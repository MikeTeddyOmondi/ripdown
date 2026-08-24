use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::Backend,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Gauge, Paragraph, Row, Table, TableState, Wrap,
    },
    Frame, Terminal,
};

use ripdown_core::engine::{detect_platform, download_url, fetch_title, resolve_output_dir};
use ripdown_core::models::{DownloadItem, DownloadStatus};
use ripdown_core::queue::{Queue, SharedQueue};

// ── Colour palette ────────────────────────────────────────────────────────────
const CYAN: Color = Color::Rgb(0, 220, 220);
const DARK_BG: Color = Color::Rgb(12, 14, 18);
const PANEL_BG: Color = Color::Rgb(18, 21, 28);
const ACCENT: Color = Color::Rgb(0, 255, 180);
const MUTED: Color = Color::Rgb(80, 90, 110);
const RED: Color = Color::Rgb(255, 80, 80);
const GOLD: Color = Color::Rgb(255, 200, 50);

// ─────────────────────────────────────────────────────────────────────────────

enum InputMode {
    Normal,
    AddingUrl,
}

struct App {
    queue: SharedQueue,
    table_state: TableState,
    input_mode: InputMode,
    url_input: String,
    audio_only: bool,
    output_dir: PathBuf,
    status_msg: Option<(String, Instant, bool)>, // (msg, born_at, is_error)
    tick: u64,
}

impl App {
    fn new(queue: SharedQueue, output_dir: PathBuf) -> Self {
        let mut ts = TableState::default();
        ts.select(None);
        Self {
            queue,
            table_state: ts,
            input_mode: InputMode::Normal,
            url_input: String::new(),
            audio_only: false,
            output_dir,
            status_msg: None,
            tick: 0,
        }
    }

    fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status_msg = Some((msg.into(), Instant::now(), is_error));
    }

    fn clear_expired_status(&mut self) {
        if let Some((_, born, _)) = &self.status_msg {
            if born.elapsed() > Duration::from_secs(4) {
                self.status_msg = None;
            }
        }
    }
}

pub async fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let output_dir = resolve_output_dir(None);
    let queue = Queue::shared();
    let mut app = App::new(queue.clone(), output_dir);

    let res = run_loop(&mut terminal, &mut app, queue).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("TUI error: {err:?}");
    }
    Ok(())
}

async fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    _queue: SharedQueue,
) -> Result<()>
where
    <B as Backend>::Error: Sync + Send + 'static,
{
    let tick_rate = Duration::from_millis(120);

    loop {
        terminal.draw(|f| ui(f, app))?;
        app.tick = app.tick.wrapping_add(1);
        app.clear_expired_status();

        if crossterm::event::poll(tick_rate)? {
            match event::read()? {
                Event::Paste(text) => {
                    if matches!(app.input_mode, InputMode::AddingUrl) {
                        app.url_input.push_str(text.trim());
                    }
                }
                Event::Key(key) => match app.input_mode {
                    InputMode::Normal => match (key.code, key.modifiers) {
                        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            break;
                        }
                        (KeyCode::Char('a'), _) | (KeyCode::Char('n'), _) => {
                            app.input_mode = InputMode::AddingUrl;
                            app.url_input.clear();
                        }
                        (KeyCode::Char('t'), _) => {
                            app.audio_only = !app.audio_only;
                        }
                        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                            let q = app.queue.read().await;
                            let len = q.items.len();
                            drop(q);
                            if len > 0 {
                                let sel = app
                                    .table_state
                                    .selected()
                                    .map(|s| (s + 1) % len)
                                    .unwrap_or(0);
                                app.table_state.select(Some(sel));
                            }
                        }
                        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                            let q = app.queue.read().await;
                            let len = q.items.len();
                            drop(q);
                            if len > 0 {
                                let sel = app
                                    .table_state
                                    .selected()
                                    .map(|s| if s == 0 { len - 1 } else { s - 1 })
                                    .unwrap_or(0);
                                app.table_state.select(Some(sel));
                            }
                        }
                        _ => {}
                    },
                    InputMode::AddingUrl => match key.code {
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                            app.url_input.clear();
                        }
                        KeyCode::Enter => {
                            let url = app.url_input.trim().to_string();
                            if url.is_empty() {
                                app.input_mode = InputMode::Normal;
                                app.url_input.clear();
                            } else {
                                enqueue_and_start(app, url).await;
                                app.input_mode = InputMode::Normal;
                                app.url_input.clear();
                            }
                        }
                        KeyCode::Char(c) => app.url_input.push(c),
                        KeyCode::Backspace => {
                            app.url_input.pop();
                        }
                        _ => {}
                    },
                },
                _ => {}
            }
        }
    }

    Ok(())
}

async fn enqueue_and_start(app: &mut App, url: String) {
    let format: &str = if app.audio_only { "bestaudio" } else { "best" };
    let item = DownloadItem::new(
        url.clone(),
        format.to_string(),
        app.audio_only,
        app.output_dir.to_string_lossy().into_owned(),
    );
    let item_id = item.id.clone();
    app.set_status(format!("Queued: {}", shorten(&url, 45)), false);

    {
        let mut q = app.queue.write().await;
        q.push(item);
    }

    // Spawn background download task
    let queue = app.queue.clone();
    let output_dir = app.output_dir.clone();
    let audio_only = app.audio_only;

    tokio::spawn(async move {
        // FetchingInfo phase
        {
            let mut q = queue.write().await;
            q.update_status(&item_id, DownloadStatus::FetchingInfo);
        }

        // Fetch title
        if let Ok((title, uploader)) = fetch_title(&url).await {
            let mut q = queue.write().await;
            q.update_title(&item_id, title, Some(uploader));
        }

        // Downloading phase
        {
            let mut q = queue.write().await;
            q.update_status(
                &item_id,
                DownloadStatus::Downloading {
                    progress: 10.0,
                    speed: "…".into(),
                    eta: "…".into(),
                },
            );
        }

        let result = download_url(&url, audio_only, &output_dir).await;

        match result {
            Ok(path) => {
                let mut q = queue.write().await;
                q.update_status(&item_id, DownloadStatus::Done { path });
            }
            Err(e) => {
                let mut q = queue.write().await;
                q.update_status(
                    &item_id,
                    DownloadStatus::Failed {
                        reason: format!("{e:#}"),
                    },
                );
            }
        }
    });
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    // Full background
    f.render_widget(Block::default().style(Style::default().bg(DARK_BG)), size);

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // header
            Constraint::Min(5),    // table
            Constraint::Length(3), // stats bar
            Constraint::Length(3), // help / input
        ])
        .split(size);

    draw_header(f, outer[0], app);
    draw_queue(f, outer[1], app);
    draw_stats(f, outer[2], app);
    draw_footer(f, outer[3], app);

    // Overlay input modal when adding URL
    if matches!(app.input_mode, InputMode::AddingUrl) {
        draw_input_modal(f, size, app);
    }
}

fn draw_header(f: &mut Frame, area: ratatui::layout::Rect, _app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(28)])
        .split(area);

    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "  ⚡ ",
                Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "rip",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "down",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  —  blazing-fast YouTube downloader",
                Style::default().fg(MUTED),
            ),
        ]),
        Line::from(vec![Span::styled(
            "  Paste a YouTube link and rip it to disk",
            Style::default().fg(MUTED),
        )]),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(MUTED))
            .border_type(BorderType::Plain),
    );

    let version = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "v0.4.0  |  [A] add  [Q] quit",
            Style::default().fg(MUTED),
        )),
    ])
    .alignment(Alignment::Right);

    f.render_widget(title, cols[0]);
    f.render_widget(version, cols[1]);
}

fn draw_queue(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let block = Block::default()
        .title(Span::styled(
            " Download Queue ",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(MUTED))
        .style(Style::default().bg(PANEL_BG));

    // Split into list + detail
    let inner = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(block.inner(area));

    f.render_widget(block, area);

    // We need an async read but we're in sync rendering — use try_read.
    // The queue is small so contention is negligible.
    let items_snapshot: Vec<DownloadItem> = {
        if let Ok(q) = app.queue.try_read() {
            q.items.clone()
        } else {
            vec![]
        }
    };

    if items_snapshot.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No downloads yet.",
                Style::default().fg(MUTED),
            )),
            Line::from(Span::styled(
                "  Press [A] to add a URL.",
                Style::default().fg(MUTED),
            )),
        ]);
        f.render_widget(empty, inner[0]);
    } else {
        // Build table rows
        let rows: Vec<Row> = items_snapshot
            .iter()
            .map(|item| {
                let status_style = match &item.status {
                    DownloadStatus::Done { .. } => {
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
                    }
                    DownloadStatus::Failed { .. } => Style::default().fg(RED),
                    DownloadStatus::Downloading { .. } => Style::default().fg(CYAN),
                    DownloadStatus::FetchingInfo => Style::default().fg(GOLD),
                    _ => Style::default().fg(MUTED),
                };

                let title = item.title.as_deref().unwrap_or(&item.url);
                let title_short = shorten(title, 38);

                let platform = item
                    .platform
                    .as_deref()
                    .unwrap_or_else(|| detect_platform(&item.url));

                let status_label = item.status.label();

                Row::new(vec![
                    Cell::from(item.id.clone()).style(Style::default().fg(MUTED)),
                    Cell::from(platform.to_string()).style(Style::default().fg(GOLD)),
                    Cell::from(title_short),
                    Cell::from(status_label).style(status_style),
                ])
            })
            .collect();

        let header = Row::new(vec!["ID", "Platform", "Title", "Status"])
            .style(
                Style::default()
                    .fg(MUTED)
                    .add_modifier(Modifier::UNDERLINED),
            )
            .height(1);

        let widths = [
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Min(0),
            Constraint::Length(12),
        ];

        let mut table_state_clone = app.table_state;
        let table = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(
                Style::default()
                    .bg(Color::Rgb(30, 40, 55))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(table, inner[0], &mut table_state_clone);

        // Detail panel for selected item
        if let Some(idx) = app.table_state.selected() {
            if let Some(item) = items_snapshot.get(idx) {
                draw_detail(f, inner[1], item);
            }
        } else {
            let hint = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  ↑↓ / j/k  select item",
                    Style::default().fg(MUTED),
                )),
            ]);
            f.render_widget(hint, inner[1]);
        }
    }
}

fn draw_detail(f: &mut Frame, area: ratatui::layout::Rect, item: &DownloadItem) {
    let pct = item.status.progress_pct();

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            " Detail ",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ID     ", Style::default().fg(MUTED)),
            Span::raw(item.id.clone()),
        ]),
        Line::from(vec![
            Span::styled("  Status ", Style::default().fg(MUTED)),
            Span::styled(
                item.status.label(),
                match &item.status {
                    DownloadStatus::Done { .. } => Style::default().fg(ACCENT),
                    DownloadStatus::Failed { .. } => Style::default().fg(RED),
                    _ => Style::default().fg(CYAN),
                },
            ),
        ]),
    ];

    if let Some(title) = &item.title {
        lines.push(Line::from(vec![
            Span::styled("  Title  ", Style::default().fg(MUTED)),
            Span::raw(shorten(title, 28)),
        ]));
    }

    if let Some(platform) = &item.platform {
        lines.push(Line::from(vec![
            Span::styled("  From   ", Style::default().fg(MUTED)),
            Span::styled(platform.clone(), Style::default().fg(GOLD)),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled("  Format ", Style::default().fg(MUTED)),
        Span::raw(if item.audio_only {
            "audio (mp3)".into()
        } else {
            item.format.clone()
        }),
    ]));

    if let DownloadStatus::Done { path } = &item.status {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Saved  ", Style::default().fg(MUTED)),
            Span::styled(
                shorten(
                    std::path::Path::new(path)
                        .file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default()
                        .as_ref(),
                    26,
                ),
                Style::default().fg(ACCENT),
            ),
        ]));
    }

    if let DownloadStatus::Failed { reason } = &item.status {
        lines.push(Line::from(""));
        for chunk in reason.lines().flat_map(|l| {
            l.chars()
                .collect::<Vec<_>>()
                .chunks(50)
                .map(|c| c.iter().collect::<String>())
                .collect::<Vec<_>>()
        }) {
            lines.push(Line::from(Span::styled(
                format!("  ✗ {chunk}"),
                Style::default().fg(RED),
            )));
        }
    }

    let detail = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(
        detail,
        area.inner(Margin {
            horizontal: 1,
            vertical: 0,
        }),
    );

    // Progress gauge (below middle of detail)
    if pct > 0.0 && pct < 100.0 && area.height > 10 {
        let gauge_area = ratatui::layout::Rect {
            x: area.x + 1,
            y: area.y + area.height - 3,
            width: area.width.saturating_sub(2),
            height: 1,
        };
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(CYAN).bg(PANEL_BG))
            .ratio(pct / 100.0)
            .label(Span::styled(
                format!("{pct:.0}%"),
                Style::default().fg(Color::White),
            ));
        f.render_widget(gauge, gauge_area);
    }
}

fn draw_stats(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let (queued, active, done, failed) = if let Ok(q) = app.queue.try_read() {
        (
            q.queued_count(),
            q.active_count(),
            q.done_count(),
            q.failed_count(),
        )
    } else {
        (0, 0, 0, 0)
    };

    let out = app.output_dir.to_string_lossy();

    let line = Line::from(vec![
        Span::styled("  QUEUED ", Style::default().fg(MUTED)),
        Span::styled(
            queued.to_string(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("   ACTIVE ", Style::default().fg(MUTED)),
        Span::styled(
            active.to_string(),
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled("   DONE ", Style::default().fg(MUTED)),
        Span::styled(
            done.to_string(),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled("   FAILED ", Style::default().fg(MUTED)),
        Span::styled(
            failed.to_string(),
            Style::default().fg(RED).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("   →  {out}"), Style::default().fg(MUTED)),
    ]);

    let stats = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(MUTED)),
    );

    f.render_widget(stats, area);
}

fn draw_footer(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    // Show status message or keybinds
    let content = if let Some((msg, _, is_error)) = &app.status_msg {
        Line::from(Span::styled(
            format!("  {msg}"),
            if *is_error {
                Style::default().fg(RED)
            } else {
                Style::default().fg(ACCENT)
            },
        ))
    } else {
        let audio_label = if app.audio_only { "ON " } else { "OFF" };
        Line::from(vec![
            Span::styled("  [A]", Style::default().fg(CYAN)),
            Span::styled(" add URL  ", Style::default().fg(MUTED)),
            Span::styled("[T]", Style::default().fg(CYAN)),
            Span::styled(
                format!(" audio-only: {audio_label}  "),
                Style::default().fg(MUTED),
            ),
            Span::styled("[↑↓]", Style::default().fg(CYAN)),
            Span::styled(" select  ", Style::default().fg(MUTED)),
            Span::styled("[Q]", Style::default().fg(CYAN)),
            Span::styled(" quit", Style::default().fg(MUTED)),
        ])
    };

    let footer = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(MUTED)),
    );

    f.render_widget(footer, area);
}

fn draw_input_modal(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let modal_w = area.width.min(70);
    let modal_h = 9u16;
    let mx = (area.width.saturating_sub(modal_w)) / 2;
    let my = (area.height.saturating_sub(modal_h)) / 2;

    let modal_area = ratatui::layout::Rect {
        x: mx,
        y: my,
        width: modal_w,
        height: modal_h,
    };

    f.render_widget(Clear, modal_area);

    let block = Block::default()
        .title(Span::styled(
            " ⚡ Add Download URL ",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(CYAN))
        .style(Style::default().bg(Color::Rgb(15, 20, 30)));

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2), // 1 for text + 1 for bottom border
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .margin(1)
        .split(inner);

    let hint = Paragraph::new(Line::from(vec![Span::styled(
        "Paste a YouTube video or playlist URL…",
        Style::default().fg(MUTED),
    )]));
    f.render_widget(hint, rows[0]);

    // Input field
    let input_display = format!("▶  {}█", app.url_input);
    let input = Paragraph::new(Span::styled(
        input_display,
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(ACCENT)),
    );
    f.render_widget(input, rows[1]);

    let audio_hint = if app.audio_only {
        Span::styled(
            "🎵 Audio-only mode is ON  (toggle with [T] before adding)",
            Style::default().fg(GOLD),
        )
    } else {
        Span::styled(
            "🎬 Video mode  (toggle audio-only with [T] before adding)",
            Style::default().fg(MUTED),
        )
    };
    f.render_widget(Paragraph::new(Line::from(audio_hint)), rows[2]);

    let keys = Paragraph::new(Line::from(vec![
        Span::styled("[Enter]", Style::default().fg(CYAN)),
        Span::styled(" confirm   ", Style::default().fg(MUTED)),
        Span::styled("[Esc]", Style::default().fg(CYAN)),
        Span::styled(" cancel", Style::default().fg(MUTED)),
    ]));
    f.render_widget(keys, rows[3]);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn shorten(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > max {
        chars[..max.saturating_sub(1)].iter().collect::<String>() + "…"
    } else {
        s.to_string()
    }
}
