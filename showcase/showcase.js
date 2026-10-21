const root = document.querySelector("[data-carousel]");

if (root) {
  const stage = root.querySelector("[data-stage]");
  const track = root.querySelector("[data-track]");
  const slides = Array.from(track.children);
  const prevButton = root.querySelector("[data-prev]");
  const nextButton = root.querySelector("[data-next]");
  const caption = root.querySelector("[data-caption]");
  const counter = root.querySelector("[data-count]");
  const dotList = root.querySelector("[data-dots]");
  const live = root.querySelector("[data-live]");
  const captions = slides.map((slide) => slide.dataset.slideCaption ?? "");

  let index = 0;

  const dots = captions.map((label, dotIndex) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "dot";
    button.setAttribute("aria-label", `Screenshot ${dotIndex + 1}: ${label}`);
    button.append(document.createElement("span"));
    button.addEventListener("click", () => go(dotIndex));
    dotList.append(button);
    return button;
  });

  function render({ announce = true } = {}) {
    track.style.transform = `translateX(${-index * 100}%)`;
    slides.forEach((slide, slideIndex) => {
      slide.setAttribute("aria-hidden", String(slideIndex !== index));
    });
    dots.forEach((dot, dotIndex) => {
      dot.setAttribute("aria-current", String(dotIndex === index));
    });
    caption.textContent = captions[index];
    counter.textContent = `${index + 1} / ${slides.length}`;
    if (announce) {
      live.textContent = `Screenshot ${index + 1} of ${slides.length}. ${captions[index]}`;
    }
  }

  function go(target) {
    index = (target + slides.length) % slides.length;
    render();
  }

  prevButton.addEventListener("click", () => go(index - 1));
  nextButton.addEventListener("click", () => go(index + 1));

  window.addEventListener("keydown", (event) => {
    if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) return;
    const target = event.target;
    if (target instanceof HTMLElement) {
      if (target.isContentEditable) return;
      if (/^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName)) return;
    }
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      go(index - 1);
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      go(index + 1);
    }
  });

  let startX = null;
  let startY = null;

  stage.addEventListener("pointerdown", (event) => {
    if (event.pointerType === "mouse") return;
    startX = event.clientX;
    startY = event.clientY;
  });

  stage.addEventListener("pointerup", (event) => {
    if (startX === null) return;
    const dx = event.clientX - startX;
    const dy = event.clientY - startY;
    startX = null;
    if (Math.abs(dx) > 40 && Math.abs(dx) > Math.abs(dy)) {
      go(index + (dx < 0 ? 1 : -1));
    }
  });

  stage.addEventListener("pointercancel", () => {
    startX = null;
  });

  render({ announce: false });
}
