//! Opt-in workload on the actual native window and its platform frame clock.
use super::*;
use std::{sync::Mutex, time::Instant};

#[derive(Clone)]
pub struct LiveProbe {
    pub started: Instant,
    /// Source length to confirm this burst was included in the submitted scene.
    pub source_bytes: usize,
    pub tool_bytes: usize,
}

#[derive(Default)]
struct Measurements {
    count: usize,
    event: Vec<(Instant, Instant)>,
    input: Vec<(Instant, Instant)>,
    pending_input: Option<(usize, Instant)>,
}

struct NativeBench {
    probes: Arc<Mutex<Vec<LiveProbe>>>,
    samples: Rc<RefCell<Measurements>>,
    streaming_sessions: HashSet<String>,
}

impl gpui::Global for NativeBench {}

pub(super) fn record_rendered_revision(app: &VelaApp, cx: &App) {
    let Some(bench) = cx.try_global::<NativeBench>() else { return; };
    let mut source = usize::MAX;
    let mut tool = usize::MAX;
    for panel in app.panels.entries().iter()
        .filter(|panel| bench.streaming_sessions.contains(&panel.session))
    {
        let blocks = if app.active_id.as_deref() == Some(panel.session.as_str()) {
            &app.blocks
        } else {
            &panel.blocks
        };
        source = source.min(blocks.iter().rev().find(|block| block.part_id.as_ref() == "stress-line")
            .map_or(0, |block| block.text.len()));
        tool = tool.min(blocks.iter().rev().find(|block| block.part_id.as_ref() == "stress-output")
            .map_or(0, |block| block.output.len()));
    }
    let now = Instant::now();
    let mut samples = bench.samples.borrow_mut();
    let probes = bench.probes.lock().expect("probe timestamps");
    while let Some(probe) = probes.get(samples.count) {
        // Once the edge replacement reaches the scene, any unseen append
        // markers were coalesced into that replacement, not presented alone.
        let matched = if probe.tool_bytes > 0 { tool >= probe.tool_bytes }
            else { source >= probe.source_bytes || tool > 0 };
        if !matched { break; }
        samples.event.push((probe.started, now));
        samples.count += 1;
    }
    if let Some((bytes, started)) = samples.pending_input
        && app.composer.len() >= bytes
    {
        samples.input.push((started, now));
        samples.pending_input = None;
    }
}

fn report(label: &str, series: impl IntoIterator<Item = Duration>) {
    let mut durations: Vec<_> = series.into_iter().collect();
    durations.sort_unstable();
    let sample = |p: f64| durations[((durations.len() - 1) as f64 * p).round() as usize].as_secs_f64() * 1000.;
    if durations.is_empty() {
        eprintln!("{label}: no samples");
    } else {
        eprintln!("{label}: n={} p50={:.3} ms p95={:.3} ms max={:.3} ms",
            durations.len(), sample(0.50), sample(0.95), sample(1.));
    }
}

pub fn run_native_transcript_bench(
    engine: Engine,
    ids: Vec<String>,
    streaming_count: usize,
    probes: Arc<Mutex<Vec<LiveProbe>>>,
    start: tokio::sync::oneshot::Sender<()>,
    done: tokio::sync::oneshot::Receiver<()>,
    open_output: bool,
) {
    let visual_check = std::env::var_os("VELA_TRANSCRIPT_VISUAL_CHECK").is_some();
    let settings = engine.settings();
    theme::set_appearance(&settings.appearance);
    gpui_platform::application().with_assets(IconAssets).run(move |cx| {
        cx.set_global(NativeBench {
            probes: probes.clone(),
            samples: Default::default(),
            streaming_sessions: ids[..streaming_count].iter().cloned().collect(),
        });
        let _ = cx.text_system().add_fonts(vec![
            std::borrow::Cow::Borrowed(CODICON_FONT),
            std::borrow::Cow::Borrowed(OPENCODE_INTER_FONT),
        ]);
        theme::initialize_source_font(cx);
        let dimensions = size(px(1680.), px(1050.));
        let bounds = Bounds::centered(None, dimensions, cx);
        cx.open_window(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(gpui::TitlebarOptions { title: Some("Vela transcript benchmark".into()), ..Default::default() }),
            ..Default::default()
        }, move |window, cx| {
            let active = ids[0].clone();
            let view = cx.new(|cx| {
                let mut app = VelaApp::new(engine, settings, None, Default::default(), cx);
                app.bootstrapped = true;
                app.loading = false;
                app.workspace_closed = false;
                app.agent_mode = ids.len() > 1;
                for id in &ids {
                    app.panels.open(crate::panels::PanelState::new(id, id)).unwrap();
                    app.ensure_panel_input(id, cx);
                }
                app.panels.activate(&active);
                app.active_id = Some(active.clone());
                if open_output {
                    for id in &ids {
                        let disclosure = format!("{id}:stress-output");
                        if *id == active {
                            app.transcript_ui.tool_open.insert(disclosure);
                        } else {
                            app.panel_transcripts.entry(id.clone()).or_default().ui.tool_open.insert(disclosure);
                        }
                    }
                }
                app.refresh_active();
                app.refresh_panel_snapshots();
                app
            });
            let owner = view.clone();
            window.spawn(cx, async move |cx| {
                let timer = cx.background_executor().clone();
                timer.timer(Duration::from_millis(700)).await;
                let (scale, viewport) = cx.update_root(|_, window, cx| {
                    owner.update(cx, |app, cx| {
                        assert!(ids.iter().all(|id| app.list_for(id).is_some_and(|list| {
                            let bounds = list.viewport_bounds();
                            bounds.size.width > px(0.) && bounds.size.height > px(0.)
                        })), "all measured sessions must have a visible transcript");
                        window.focus(&app.composer_focus, cx);
                    });
                    (window.scale_factor(), window.viewport_size())
                }).unwrap();
                eprintln!("native viewport {viewport:?}, scale {scale}x, {} panels", ids.len());
                gpui::profiler::set_frame_trace_enabled(true);
                let mut frames = gpui::profiler::FrameTimingCollector::new();
                let _ = start.send(());
                let mut done = done;
                let mut reading_anchor = None;
                let mut checked_scroll = false;
                loop {
                    tokio::select! {
                        _ = &mut done => break,
                        _ = timer.timer(Duration::from_millis(160)) => {
                            cx.update_root(|_, window, cx| {
                                let samples = cx.global::<NativeBench>().samples.clone();
                                let count = samples.borrow().count;
                                if visual_check && count >= 12 && !checked_scroll {
                                    if let Some(anchor) = reading_anchor {
                                        owner.update(cx, |app, cx| {
                                            assert!(!app.stick, "scroll left tail-follow mode");
                                            let offset = app.transcript_list.logical_scroll_top();
                                            assert_eq!((offset.item_ix, offset.offset_in_item), anchor);
                                            assert!(ids[1..].iter().all(|id| app.list_for(id)
                                                .is_some_and(|list| list.is_following_tail())));
                                            app.scroll_transcript_to_bottom(cx);
                                        });
                                        checked_scroll = true;
                                        eprintln!("native reading anchor and sibling follow verified");
                                    } else {
                                        let list = owner.read(cx).transcript_list.clone();
                                        window.dispatch_event(gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                                            position: list.viewport_bounds().center(),
                                            delta: gpui::ScrollDelta::Pixels(point(px(0.), px(600.))),
                                            ..Default::default()
                                        }), cx);
                                        let offset = list.logical_scroll_top();
                                        reading_anchor = Some((offset.item_ix, offset.offset_in_item));
                                    }
                                }
                                let bytes = owner.read(cx).composer.len() + 1;
                                if samples.borrow().pending_input.is_none() {
                                    samples.borrow_mut().pending_input = Some((bytes, Instant::now()));
                                    let _ = window.dispatch_keystroke(gpui::Keystroke::parse("x").unwrap(), cx);
                                }
                            }).unwrap();
                        }
                    }
                }
                assert!(!visual_check || checked_scroll, "visual fixture did not exercise native scroll");
                timer.timer(Duration::from_millis(600)).await;
                let frames = frames.collect_unseen();
                cx.update_root(|_, _, cx| {
                    let bench = cx.global::<NativeBench>();
                    let samples = bench.samples.borrow();
                    let probes = probes.lock().unwrap();
                    let edge = probes.iter().find(|probe| probe.tool_bytes > 0).map(|probe| probe.started);
                    let ordinary = |frame: &&gpui::profiler::FrameTiming| edge.is_none_or(|edge| frame.draw_start < edge);
                    report("ordinary native window draw work", frames.iter().filter(ordinary).map(|frame| frame.draw_duration()));
                    report("edge native window draw work", frames.iter().filter(|frame| !ordinary(frame)).map(|frame| frame.draw_duration()));
                    let through_draw = |&(started, rendered): &(Instant, Instant)| frames.iter()
                        .find(|frame| frame.draw_start <= rendered && rendered <= frame.draw_end)
                        .map(|frame| frame.draw_end.duration_since(started));
                    report("ordinary SSE burst to native scene recorded",
                        samples.event.iter().filter(|(started, _)| edge != Some(*started)).filter_map(through_draw));
                    report("final payload to native scene recorded",
                        samples.event.iter().filter(|(started, _)| edge == Some(*started)).filter_map(through_draw));
                    report("app-scoped input to native scene recorded", samples.input.iter().filter_map(through_draw));
                    eprintln!("observed source markers {}/{}", samples.count, probes.len());
                    if visual_check {
                        assert_eq!(samples.count, probes.len());
                        assert!(!samples.input.is_empty(), "composer input reached a native scene");
                    }
                }).unwrap();
                if visual_check {
                    cx.update_root(|_, window, cx| {
                        let mut right = px(0.);
                        let app = owner.read(cx);
                        for id in &ids {
                            let list = app.list_for(id).unwrap();
                            let rows = if app.active_id.as_deref() == Some(id.as_str()) {
                                &app.transcript_rows
                            } else {
                                &app.panel_transcripts[id].rows
                            };
                            let tool_index = rows.iter().position(|row| row.id.contains("stress-output"))
                                .expect("the settled tool has a transcript row");
                            let bounds = list.viewport_bounds();
                            let tool = list.bounds_for_item(tool_index)
                                .expect("settled tool has been measured");
                            let footer = list.bounds_for_item(list.item_count() - 1)
                                .expect("settled footer has been measured");
                            assert!(bounds.size.height >= window.viewport_size().height / 4.);
                            assert!(bounds.bottom() < window.viewport_size().height);
                            assert!(tool.top() >= bounds.top() && tool.bottom() <= bounds.bottom());
                            assert!(footer.top() >= bounds.top() && footer.bottom() <= bounds.bottom());
                            right = right.max(bounds.right());
                        }
                        assert!(right >= window.viewport_size().width - px(40.));
                        eprintln!("settled tool and footer fit in all eight native tiles");
                    }).unwrap();
                    eprintln!("visual fixture settled");
                    timer.timer(Duration::from_secs(8)).await;
                }
                cx.update_root(|_, _, cx| cx.quit()).unwrap();
            }).detach();
            view
        }).expect("open the native benchmark window");
        cx.activate(true);
    });
}
