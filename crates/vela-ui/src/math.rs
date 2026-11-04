//! Bounded native math rendering for Markdown. Source uses KaTeX; this module
//! intentionally implements a readable TeX subset (Greek/operator symbols,
//! scripts, fractions, roots and text groups) as Unicode, without attempting a
//! full TeX layout engine.

pub const MATH_FONT: &str = "STIX Two Math";

pub fn render(tex: &str) -> String {
    let mut parser = Parser {
        chars: tex.chars().collect(),
        index: 0,
    };
    parser.sequence(None).trim().to_owned()
}

struct Parser {
    chars: Vec<char>,
    index: usize,
}

impl Parser {
    fn sequence(&mut self, stop: Option<char>) -> String {
        let mut out = String::new();
        while self.index < self.chars.len() {
            let ch = self.chars[self.index];
            if Some(ch) == stop {
                break;
            }
            self.index += 1;
            match ch {
                '{' => {
                    out.push_str(&self.sequence(Some('}')));
                    if self.peek() == Some('}') {
                        self.index += 1;
                    }
                }
                '}' => break,
                '\\' => out.push_str(&self.command()),
                '^' => {
                    let value = self.argument();
                    out.push_str(&script(&value, true));
                }
                '_' => {
                    let value = self.argument();
                    out.push_str(&script(&value, false));
                }
                '$' => {}
                '~' => out.push('\u{00A0}'),
                ch if ch.is_whitespace() => {
                    if !out.ends_with(' ') {
                        out.push(' ');
                    }
                }
                other => out.push(other),
            }
        }
        out
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn argument(&mut self) -> String {
        while self.peek().is_some_and(char::is_whitespace) {
            self.index += 1;
        }
        match self.peek() {
            Some('{') => {
                self.index += 1;
                let value = self.sequence(Some('}'));
                if self.peek() == Some('}') {
                    self.index += 1;
                }
                value
            }
            Some('\\') => {
                self.index += 1;
                self.command()
            }
            Some(_) => {
                let ch = self.chars[self.index];
                self.index += 1;
                ch.to_string()
            }
            None => String::new(),
        }
    }

    /// Read a braced argument without interpreting its contents. This keeps
    /// row separators and nested groups intact for `\substack`.
    fn raw_argument(&mut self) -> String {
        while self.peek().is_some_and(char::is_whitespace) {
            self.index += 1;
        }
        if self.peek() != Some('{') {
            return self.argument();
        }
        self.index += 1;
        let start = self.index;
        let mut depth = 1usize;
        while self.index < self.chars.len() {
            match self.chars[self.index] {
                '\\' => {
                    self.index += 1;
                    if self.index < self.chars.len() {
                        self.index += 1;
                    }
                }
                '{' => {
                    depth += 1;
                    self.index += 1;
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let raw = self.chars[start..self.index].iter().collect();
                        self.index += 1;
                        return raw;
                    }
                    self.index += 1;
                }
                _ => self.index += 1,
            }
        }
        self.chars[start..].iter().collect()
    }

    fn environment(&mut self) -> String {
        let name = self.argument();
        if matches!(name.as_str(), "array" | "alignat" | "alignedat") {
            let _ = self.argument();
        }
        let marker = format!("\\end{{{name}}}");
        let tail = self.chars[self.index..].iter().collect::<String>();
        let raw = if let Some(byte) = tail.find(&marker) {
            let raw = tail[..byte].to_owned();
            self.index += raw.chars().count() + marker.chars().count();
            raw
        } else {
            self.index = self.chars.len();
            tail
        };
        matrix(&raw, &name)
    }

    fn optional_bracket(&mut self) -> Option<String> {
        while self.peek().is_some_and(char::is_whitespace) {
            self.index += 1;
        }
        if self.peek() != Some('[') {
            return None;
        }
        self.index += 1;
        let value = self.sequence(Some(']'));
        if self.peek() == Some(']') {
            self.index += 1;
        }
        Some(value)
    }

    fn command(&mut self) -> String {
        let Some(first) = self.peek() else {
            return "\\".into();
        };
        if !first.is_alphabetic() {
            self.index += 1;
            return match first {
                ' ' => " ".into(),
                ',' => "\u{2009}".into(),
                ';' | ':' => " ".into(),
                '!' => String::new(),
                '\\' => " ".into(),
                '{' => "{".into(),
                '}' => "}".into(),
                other => other.to_string(),
            };
        }
        let start = self.index;
        while self.peek().is_some_and(char::is_alphabetic) {
            self.index += 1;
        }
        let name = self.chars[start..self.index].iter().collect::<String>();
        match name.as_str() {
            "frac" | "dfrac" | "tfrac" => {
                let numerator = self.argument();
                let denominator = self.argument();
                fraction(&numerator, &denominator)
            }
            "binom" | "dbinom" | "tbinom" => {
                let numerator = self.argument();
                let denominator = self.argument();
                binomial(&numerator, &denominator)
            }
            "overset" | "stackrel" => {
                let annotation = self.argument();
                let base = self.argument();
                format!("{base}{}", script(&annotation, true))
            }
            "underset" => {
                let annotation = self.argument();
                let base = self.argument();
                format!("{base}{}", script(&annotation, false))
            }
            "substack" => {
                let raw = self.raw_argument();
                raw.split("\\\\")
                    .map(render)
                    .map(|row| row.trim().to_owned())
                    .filter(|row| !row.is_empty())
                    .collect::<Vec<_>>()
                    .join("; ")
            }
            "boldsymbol" | "bm" | "pmb" => self.argument(),
            "begin" => self.environment(),
            "end" => {
                let _ = self.argument();
                String::new()
            }
            "hat" | "widehat" => accent(&self.argument(), '\u{0302}'),
            "bar" => accent(&self.argument(), '\u{0304}'),
            "vec" => accent(&self.argument(), '\u{20D7}'),
            "dot" => accent(&self.argument(), '\u{0307}'),
            "ddot" => accent(&self.argument(), '\u{0308}'),
            "tilde" | "widetilde" => accent(&self.argument(), '\u{0303}'),
            "overline" => overline(&self.argument(), '\u{0305}'),
            "underline" => overline(&self.argument(), '\u{0332}'),
            "overrightarrow" => accent(&self.argument(), '\u{20D7}'),
            "overleftarrow" => accent(&self.argument(), '\u{20D6}'),
            "overleftrightarrow" => accent(&self.argument(), '\u{20E1}'),
            "underrightarrow" => accent(&self.argument(), '\u{20D7}'),
            "underleftarrow" => accent(&self.argument(), '\u{20D6}'),
            "underbrace" => format!("{}⏟", self.argument()),
            "overbrace" => format!("{}⏞", self.argument()),
            "hspace" | "hspace*" | "mspace" => {
                let amount = self.argument();
                spacing(&amount)
            }
            "sqrt" => {
                let index = self.optional_bracket();
                let value = self.argument();
                let root = index
                    .filter(|index| !index.trim().is_empty())
                    .map(|index| script(&index, true))
                    .unwrap_or_default();
                if atomic(&value) {
                    format!("{root}√{value}")
                } else {
                    format!("{root}√({value})")
                }
            }
            "log" | "ln" | "exp" | "sin" | "cos" | "tan" | "sec" | "csc" | "cot"
            | "arcsin" | "arccos" | "arctan" | "sinh" | "cosh" | "tanh" | "lim"
            | "max" | "min" | "sup" | "inf" | "det" | "gcd" => name,
            "displaystyle" | "textstyle" | "scriptstyle" | "scriptscriptstyle" => String::new(),
            "phantom" | "hphantom" | "vphantom" => {
                let _ = self.argument();
                String::new()
            }
            "boxed" | "fbox" | "framebox" => format!("[{}]", self.argument()),
            "cancel" | "bcancel" | "xcancel" | "sout" => strike(&self.argument()),
            "big" | "Big" | "bigg" | "Bigg" | "bigl" | "bigr" | "Bigl" | "Bigr"
            | "biggl" | "biggr" | "Biggl" | "Biggr" | "bigm" | "Bigm" | "middle" => {
                self.argument()
            }
            "mathbin" | "mathrel" | "mathop" | "mathord" | "mathopen" | "mathclose"
            | "mathpunct" | "mathinner" | "hbox" => self.argument(),
            "pmod" => format!(" (mod {})", self.argument()),
            "bmod" | "mod" => " mod ".into(),
            "textcolor" => {
                let _ = self.argument();
                self.argument()
            }
            "colorbox" => {
                let _ = self.argument();
                self.argument()
            }
            "fcolorbox" => {
                let _ = self.argument();
                let _ = self.argument();
                self.argument()
            }
            "color" => {
                let _ = self.argument();
                String::new()
            }
            "mathbb" => blackboard(&self.argument()),
            "text" | "mathrm" | "mathbf" | "mathit" | "operatorname" | "textbf" | "textit"
            | "textsf" | "texttt" | "textnormal" | "emph" => self.argument(),
            "left" | "right" => {
                while self.peek().is_some_and(char::is_whitespace) {
                    self.index += 1;
                }
                if let Some(delimiter) = self.peek()
                    && delimiter != '\\'
                {
                    self.index += 1;
                    if delimiter == '.' {
                        String::new()
                    } else {
                        delimiter.to_string()
                    }
                } else {
                    String::new()
                }
            }
            "quad" => "  ".into(),
            "qquad" => "    ".into(),
            _ => symbol(&name).map_or_else(|| format!("\\{name}"), str::to_owned),
        }
    }
}

fn matrix(raw: &str, name: &str) -> String {
    let rows = raw
        .split("\\\\")
        .map(|row| {
            row.split('&')
                .map(render)
                .filter(|cell| !cell.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>()
        .join("; ");
    match name {
        "pmatrix" => format!("({rows})"),
        "bmatrix" => format!("[{rows}]"),
        "vmatrix" => format!("|{rows}|"),
        "Bmatrix" | "cases" => format!("{{{rows}}}"),
        "aligned" | "align" | "alignedat" | "alignat" | "gathered" | "split" => rows,
        _ => rows,
    }
}

fn accent(value: &str, mark: char) -> String {
    let mut out = value.to_owned();
    if let Some(last) = out.pop() {
        if !last.is_whitespace() {
            out.push(last);
            out.push(mark);
        } else {
            out.push(last);
        }
    }
    out
}

fn overline(value: &str, mark: char) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        out.push(ch);
        if !ch.is_whitespace() {
            out.push(mark);
        }
    }
    out
}

fn spacing(amount: &str) -> String {
    let count = amount
        .chars()
        .find(|ch| ch.is_ascii_digit())
        .and_then(|ch| ch.to_digit(10))
        .unwrap_or(1)
        .clamp(1, 8) as usize;
    " ".repeat(count)
}

fn atomic(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|ch| ch.is_alphanumeric() || matches!(ch, '.' | ','))
}

fn binomial(numerator: &str, denominator: &str) -> String {
    let numerator = if atomic(numerator) {
        numerator.to_owned()
    } else {
        format!("({numerator})")
    };
    let denominator = if atomic(denominator) {
        denominator.to_owned()
    } else {
        format!("({denominator})")
    };
    format!("({numerator} choose {denominator})")
}

fn strike(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        out.push(ch);
        if !ch.is_whitespace() {
            out.push('\u{0338}');
        }
    }
    out
}

fn blackboard(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'C' => 'ℂ',
            'H' => 'ℍ',
            'N' => 'ℕ',
            'P' => 'ℙ',
            'Q' => 'ℚ',
            'R' => 'ℝ',
            'Z' => 'ℤ',
            other => other,
        })
        .collect()
}

fn fraction(numerator: &str, denominator: &str) -> String {
    let numerator = if atomic(numerator) {
        numerator.to_owned()
    } else {
        format!("({numerator})")
    };
    let denominator = if atomic(denominator) {
        denominator.to_owned()
    } else {
        format!("({denominator})")
    };
    format!("{numerator}\u{2044}{denominator}")
}

fn script(value: &str, superscript: bool) -> String {
    let table = if superscript { SUPERSCRIPT } else { SUBSCRIPT };
    let mapped = value
        .chars()
        .map(|ch| table.iter().find(|(from, _)| *from == ch).map(|(_, to)| *to))
        .collect::<Option<String>>();
    match mapped {
        Some(mapped) => mapped,
        None if superscript => format!("^({value})"),
        None => format!("_({value})"),
    }
}

fn symbol(name: &str) -> Option<&'static str> {
    Some(match name {
        "alpha" => "α",
        "beta" => "β",
        "gamma" => "γ",
        "delta" => "δ",
        "epsilon" => "ε",
        "varepsilon" => "ε",
        "zeta" => "ζ",
        "eta" => "η",
        "theta" => "θ",
        "iota" => "ι",
        "kappa" => "κ",
        "lambda" => "λ",
        "mu" => "μ",
        "nu" => "ν",
        "xi" => "ξ",
        "pi" => "π",
        "rho" => "ρ",
        "sigma" => "σ",
        "tau" => "τ",
        "upsilon" => "υ",
        "phi" => "φ",
        "chi" => "χ",
        "psi" => "ψ",
        "omega" => "ω",
        "Gamma" => "Γ",
        "Delta" => "Δ",
        "Theta" => "Θ",
        "Lambda" => "Λ",
        "Xi" => "Ξ",
        "Pi" => "Π",
        "Sigma" => "Σ",
        "Phi" => "Φ",
        "Psi" => "Ψ",
        "Omega" => "Ω",
        "times" => "×",
        "cdot" => "·",
        "pm" => "±",
        "mp" => "∓",
        "le" | "leq" => "≤",
        "ge" | "geq" => "≥",
        "ne" | "neq" => "≠",
        "approx" => "≈",
        "equiv" => "≡",
        "ll" => "≪",
        "gg" => "≫",
        "sim" => "∼",
        "simeq" => "≃",
        "cong" => "≅",
        "propto" => "∝",
        "perp" => "⊥",
        "parallel" => "∥",
        "mid" => "∣",
        "nmid" => "∤",
        "subset" => "⊂",
        "supset" => "⊃",
        "subseteq" => "⊆",
        "supseteq" => "⊇",
        "subsetneq" => "⊊",
        "supsetneq" => "⊋",
        "prec" => "≺",
        "succ" => "≻",
        "preceq" => "⪯",
        "succeq" => "⪰",
        "asymp" => "≍",
        "bowtie" => "⋈",
        "doteq" => "≐",
        "models" => "⊨",
        "vdash" => "⊢",
        "dashv" => "⊣",
        "infty" => "∞",
        "sum" => "∑",
        "prod" => "∏",
        "int" => "∫",
        "iint" => "∬",
        "iiint" => "∭",
        "oint" => "∮",
        "bigcup" => "⋃",
        "bigcap" => "⋂",
        "partial" => "∂",
        "nabla" => "∇",
        "to" | "rightarrow" => "→",
        "leftarrow" => "←",
        "leftrightarrow" => "↔",
        "Rightarrow" => "⇒",
        "Leftarrow" => "⇐",
        "Leftrightarrow" => "⇔",
        "mapsto" => "↦",
        "longrightarrow" => "⟶",
        "longleftarrow" => "⟵",
        "Longrightarrow" => "⟹",
        "Longleftarrow" => "⟸",
        "hookrightarrow" => "↪",
        "hookleftarrow" => "↩",
        "uparrow" => "↑",
        "downarrow" => "↓",
        "updownarrow" => "↕",
        "Uparrow" => "⇑",
        "Downarrow" => "⇓",
        "nearrow" => "↗",
        "searrow" => "↘",
        "swarrow" => "↙",
        "nwarrow" => "↖",
        "in" => "∈",
        "notin" => "∉",
        "cup" => "∪",
        "cap" => "∩",
        "forall" => "∀",
        "exists" => "∃",
        "ell" => "ℓ",
        "hbar" => "ℏ",
        "Re" => "ℜ",
        "Im" => "ℑ",
        "aleph" => "ℵ",
        "emptyset" | "varnothing" => "∅",
        "angle" => "∠",
        "triangle" => "△",
        "square" => "□",
        "bullet" => "•",
        "star" => "⋆",
        "ast" => "∗",
        "oplus" => "⊕",
        "otimes" => "⊗",
        "odot" => "⊙",
        "circ" => "∘",
        "prime" => "′",
        "backslash" => "\\",
        "therefore" => "∴",
        "because" => "∵",
        "ldots" => "…",
        "cdots" => "⋯",
        "dots" => "…",
        "vdots" => "⋮",
        "ddots" => "⋱",
        "degree" => "°",
        _ => return None,
    })
}

const SUPERSCRIPT: &[(char, char)] = &[
    ('0', '⁰'),
    ('1', '¹'),
    ('2', '²'),
    ('3', '³'),
    ('4', '⁴'),
    ('5', '⁵'),
    ('6', '⁶'),
    ('7', '⁷'),
    ('8', '⁸'),
    ('9', '⁹'),
    ('+', '⁺'),
    ('-', '⁻'),
    ('=', '⁼'),
    ('(', '⁽'),
    (')', '⁾'),
    ('n', 'ⁿ'),
    ('i', 'ⁱ'),
    ('x', 'ˣ'),
    ('y', 'ʸ'),
    ('a', 'ᵃ'),
    ('b', 'ᵇ'),
    ('c', 'ᶜ'),
    ('d', 'ᵈ'),
    ('e', 'ᵉ'),
    ('f', 'ᶠ'),
    ('g', 'ᵍ'),
    ('h', 'ʰ'),
    ('j', 'ʲ'),
    ('k', 'ᵏ'),
    ('l', 'ˡ'),
    ('m', 'ᵐ'),
    ('o', 'ᵒ'),
    ('p', 'ᵖ'),
    ('r', 'ʳ'),
    ('s', 'ˢ'),
    ('t', 'ᵗ'),
    ('u', 'ᵘ'),
    ('v', 'ᵛ'),
    ('w', 'ʷ'),
    ('z', 'ᶻ'),
];

const SUBSCRIPT: &[(char, char)] = &[
    ('0', '₀'),
    ('1', '₁'),
    ('2', '₂'),
    ('3', '₃'),
    ('4', '₄'),
    ('5', '₅'),
    ('6', '₆'),
    ('7', '₇'),
    ('8', '₈'),
    ('9', '₉'),
    ('+', '₊'),
    ('-', '₋'),
    ('=', '₌'),
    ('(', '₍'),
    (')', '₎'),
    ('a', 'ₐ'),
    ('e', 'ₑ'),
    ('h', 'ₕ'),
    ('i', 'ᵢ'),
    ('j', 'ⱼ'),
    ('k', 'ₖ'),
    ('l', 'ₗ'),
    ('m', 'ₘ'),
    ('n', 'ₙ'),
    ('o', 'ₒ'),
    ('p', 'ₚ'),
    ('r', 'ᵣ'),
    ('s', 'ₛ'),
    ('t', 'ₜ'),
    ('u', 'ᵤ'),
    ('v', 'ᵥ'),
    ('x', 'ₓ'),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_symbols_scripts_fractions_roots_and_text() {
        assert_eq!(render("E = mc^2"), "E = mc²");
        assert_eq!(
            render(r"\alpha + \beta \times \frac{1}{2}"),
            "α + β × 1⁄2"
        );
        assert_eq!(render(r"\sqrt{x^2 + y^2}"), "√(x² + y²)");
        assert_eq!(render(r"x_i^2"), "xᵢ²");
        assert_eq!(render(r"\text{if } x \le 1"), "if x ≤ 1");
        assert_eq!(render(r"\left( a \right)"), "( a )");
    }

    #[test]
    fn renders_matrices_accents_and_tex_spacing() {
        assert_eq!(
            render(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}"),
            "(a b; c d)"
        );
        assert_eq!(
            render(r"\begin{bmatrix} 1 & 2 \\ 3 & 4 \end{bmatrix}"),
            "[1 2; 3 4]"
        );
        assert_eq!(render(r"\hat{x} + \bar{y} + \vec{v}"), "x̂ + ȳ + v⃗");
        assert_eq!(render(r"\overline{AB}"), "A̅B̅");
        assert_eq!(render(r"a~b"), "a\u{00A0}b");
        assert_eq!(render(r"a\,b"), "a\u{2009}b");
        assert_eq!(render(r"a\hspace{2em}b"), "a  b");
    }

    #[test]
    fn renders_aligned_operators_binomials_root_indices_and_style_switches() {
        assert_eq!(
            render(r"\binom{n}{k} + \binom{n+1}{2}"),
            "(n choose k) + ((n+1) choose 2)"
        );
        assert_eq!(
            render(r"\log x + \sin y + \lim_{n} a_n"),
            "log x + sin y + limₙ aₙ"
        );
        assert_eq!(render(r"\sqrt[3]{x}"), "³√x");
        assert_eq!(
            render(r"\begin{aligned} a &= b \\ c &= d \end{aligned}"),
            "a = b; c = d"
        );
        assert_eq!(render(r"\displaystyle x"), "x");
        assert_eq!(render(r"\left. x \right|"), "x |");
        assert_eq!(render(r"\phantom{x}y"), "y");
    }

    #[test]
    fn renders_blackboard_boxed_mod_and_color_commands() {
        assert_eq!(render(r"\mathbb{R} \subset \mathbb{C}"), "ℝ ⊂ ℂ");
        assert_eq!(render(r"\boxed{x + 1}"), "[x + 1]");
        assert_eq!(render(r"a \pmod{n} b \bmod c"), "a  (mod n) b  mod c");
        assert_eq!(render(r"\textcolor{red}{x} + \color{blue} y"), "x + y");
        assert_eq!(render(r"\colorbox{red}{x}"), "x");
    }

    #[test]
    fn renders_common_relations_arrows_and_symbols() {
        assert_eq!(
            render(r"a \ll b \gg c \sim d \simeq e \cong f \propto g"),
            "a ≪ b ≫ c ∼ d ≃ e ≅ f ∝ g"
        );
        assert_eq!(
            render(r"A \subseteq B \supseteq C \subsetneq D \supsetneq E"),
            "A ⊆ B ⊇ C ⊊ D ⊋ E"
        );
        assert_eq!(
            render(r"x \mapsto y \Leftrightarrow z \Longrightarrow w \hookrightarrow v"),
            "x ↦ y ⇔ z ⟹ w ↪ v"
        );
        assert_eq!(
            render(r"\ell + \hbar + \aleph + \emptyset + \angle + \square"),
            "ℓ + ℏ + ℵ + ∅ + ∠ + □"
        );
        assert_eq!(render(r"\bigcup A \bigcap B \oint C \vdots \ddots"), "⋃ A ⋂ B ∮ C ⋮ ⋱");
    }

    #[test]
    fn renders_over_and_under_braces_and_arrows() {
        assert_eq!(render(r"\overbrace{a+b}^{n}"), "a+b⏞ⁿ");
        assert_eq!(render(r"\underbrace{x_1 + x_2}_{n}"), "x₁ + x₂⏟ₙ");
        assert_eq!(render(r"\overrightarrow{AB} + \overleftarrow{CD}"), "AB⃗ + CD⃖");
        assert_eq!(render(r"\underrightarrow{AB}"), "AB⃗");
    }

    #[test]
    fn renders_big_delimiters_cancel_and_text_style_commands() {
        assert_eq!(render(r"\big( x \big) + \Big[ y \Big]"), "( x ) + [ y ]");
        assert_eq!(
            render(r"\cancel{x+y} + \sout{old}"),
            "x̸+̸y̸ + o̸l̸d̸"
        );
        assert_eq!(
            render(r"\textbf{bold} + \textit{italic} + \texttt{code}"),
            "bold + italic + code"
        );
        assert_eq!(render(r"\mathrel{=} \mathbin{+}"), "= +");
        assert_eq!(render(r"\fbox{x}"), "[x]");
    }

    #[test]
    fn renders_stacked_annotations_substacks_and_bold_symbols() {
        assert_eq!(render(r"\overset{a}{=}"), "=ᵃ");
        assert_eq!(render(r"\underset{n}{\lim}"), "limₙ");
        assert_eq!(render(r"\stackrel{def}{=}"), "=ᵈᵉᶠ");
        assert_eq!(
            render(r"\substack{a \\ b \\ c}"),
            "a; b; c"
        );
        assert_eq!(
            render(r"\substack{\text{if } x \\ \text{else } y}"),
            "if x; else y"
        );
        assert_eq!(
            render(r"\boldsymbol{\alpha} + \pmb{x} + \bm{y}"),
            "α + x + y"
        );
    }

    #[test]
    fn unmatched_groups_and_unknown_commands_stay_readable() {
        assert_eq!(render(r"\unknown{x}"), r"\unknownx");
        assert_eq!(render("{x + y"), "x + y");
        assert_eq!(render(r"\frac{a+b}{c}"), "(a+b)⁄c");
    }
}
