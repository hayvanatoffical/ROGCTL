//! Turns `rogctl.log` into a readable HTML report instead of a terminal dump.
//!
//! The log format is line-oriented text meant for tailing, not analysis - this
//! module is the other half: parse it back into sessions, compute the same
//! findings a human would (throttling without heat, mode flapping, fruitless
//! RAM passes), and render one self-contained HTML file with charts. No JS
//! framework, no network fetch - it has to open and render offline from a
//! double-click.

use std::path::PathBuf;

#[derive(Debug, Clone)]
struct SampleRow {
    t: f32,
    mode: String,
    cpu_c: f32,
    gpu_c: f32,
    cpu_pct: f32,
    gpu_pct: f32,
    ceiling: u32,
    fan_rpm: u32,
}

#[derive(Debug, Clone)]
struct ModeChange {
    t: f32,
    mode: String,
    profile: Option<String>,
    power: String,
    fan_idle: u32,
    fan_max: u32,
    gpu_floor: u32,
    gpu_ceiling: u32,
    cpu_target: u32,
    gpu_target: u32,
}

#[derive(Debug, Clone)]
struct ReclaimEvent {
    t: f32,
    trigger: String,
    depth: String,
    freed_mb: i64,
}

#[derive(Debug, Default)]
struct Session {
    forced: bool,
    ended_clean: bool,
    samples: Vec<SampleRow>,
    changes: Vec<ModeChange>,
    reclaims: Vec<ReclaimEvent>,
    warnings: Vec<String>,
}

const KNOWN_MODES: [&str; 6] = ["bosta", "film", "ofis", "hafif oyun", "AAA oyun", "render"];

fn parse_sample(line: &str) -> Option<SampleRow> {
    let toks: Vec<&str> = line.split_whitespace().collect();
    let (mode, rest): (String, &[&str]) = match toks.len() {
        9 => (format!("{} {}", toks[1], toks[2]), &toks[3..]),
        8 => (toks[1].to_string(), &toks[2..]),
        _ => return None,
    };
    if !KNOWN_MODES.contains(&mode.as_str()) {
        return None;
    }
    Some(SampleRow {
        t: toks[0].parse().ok()?,
        mode,
        cpu_c: rest[0].parse().ok()?,
        gpu_c: rest[1].parse().ok()?,
        cpu_pct: rest[2].trim_end_matches('%').parse().ok()?,
        gpu_pct: rest[3].trim_end_matches('%').parse().ok()?,
        ceiling: rest[4].parse().ok()?,
        fan_rpm: rest[5].parse().ok()?,
    })
}

fn parse_mode_change(line: &str) -> Option<ModeChange> {
    let line = line.trim();
    let (t_part, rest) = line.split_once("s] --> mod: ")?;
    let t: f32 = t_part.trim_start_matches('[').trim().parse().ok()?;

    let (before_paren, paren) = rest.split_once("  (")?;
    let paren = paren.trim_end_matches(')');
    let (mode, profile) = match before_paren.split_once(" [profil: ") {
        Some((m, p)) => (m.trim().to_string(), Some(p.trim_end_matches(']').to_string())),
        None => (before_paren.trim().to_string(), None),
    };

    let parts: Vec<&str> = paren.split(", ").collect();
    let power = parts.first()?.to_string();
    let fan_part = parts.get(1)?.trim_start_matches("fan ").trim_end_matches('%');
    let (fi, fm) = fan_part.split_once('-')?;
    let gpu_part = parts.get(2)?.trim_start_matches("GPU ").trim_end_matches("MHz");
    let (gf, gc) = gpu_part.split_once('-')?;
    let hedef = parts.get(3)?.trim_start_matches("hedef ");
    let (cpu_t, gpu_t) = hedef.split_once(" / ")?;

    Some(ModeChange {
        t,
        mode,
        profile,
        power,
        fan_idle: fi.parse().ok()?,
        fan_max: fm.parse().ok()?,
        gpu_floor: gf.parse().ok()?,
        gpu_ceiling: gc.parse().ok()?,
        cpu_target: cpu_t.trim_start_matches("CPU ").trim_end_matches('C').parse().ok()?,
        gpu_target: gpu_t.trim_start_matches("GPU ").trim_end_matches('C').parse().ok()?,
    })
}

fn parse_reclaim(line: &str) -> Option<ReclaimEvent> {
    let line = line.trim();
    if !line.contains("RAM geri kazanildi") {
        return None;
    }
    let (t_part, rest) = line.split_once("s] [")?;
    let t: f32 = t_part.trim_start_matches('[').trim().parse().ok()?;
    let (trigger, rest) = rest.split_once("] RAM geri kazanildi (")?;
    let (depth, rest) = rest.split_once("): bos ")?;
    let (_before, rest) = rest.split_once(" -> ")?;
    let (_after, rest) = rest.split_once(" MB (+")?;
    let (freed_str, _rest) = rest.split_once(" MB), standby ")?;
    Some(ReclaimEvent {
        t,
        trigger: trigger.to_string(),
        depth: depth.to_string(),
        freed_mb: freed_str.trim().parse().ok()?,
    })
}

fn parse_log(text: &str) -> Vec<Session> {
    let mut sessions = Vec::new();
    let mut cur: Option<Session> = None;

    for raw in text.lines() {
        let line = raw.trim_end();
        if line.starts_with("--- rogctl basladi") {
            if let Some(s) = cur.take() {
                sessions.push(s);
            }
            cur = Some(Session {
                forced: line.contains("DAYATILMIS"),
                ..Default::default()
            });
            continue;
        }
        let Some(s) = cur.as_mut() else { continue };

        if line.starts_with("--- rogctl durdu") {
            s.ended_clean = true;
            continue;
        }
        if let Some(r) = parse_reclaim(line) {
            s.reclaims.push(r);
            continue;
        }
        if let Some(m) = parse_mode_change(line) {
            s.changes.push(m);
            continue;
        }
        if line.contains("[!]") {
            s.warnings.push(line.trim_start_matches("[!]").trim().to_string());
            continue;
        }
        if let Some(row) = parse_sample(line) {
            s.samples.push(row);
        }
    }
    if let Some(s) = cur.take() {
        sessions.push(s);
    }
    // A session with no samples and no changes is a daemon that started and
    // was immediately replaced (rapid restarts during install/testing) - not
    // worth a card of its own.
    sessions.retain(|s| !s.samples.is_empty() || !s.changes.is_empty());
    sessions
}

// ---------------------------------------------------------------------------
// Findings
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Severity {
    Kritik,
    Uyari,
    Bilgi,
    Iyi,
}

impl Severity {
    fn tag(self) -> &'static str {
        match self {
            Severity::Kritik => "kritik",
            Severity::Uyari => "uyari",
            Severity::Bilgi => "bilgi",
            Severity::Iyi => "iyi",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Severity::Kritik => "KRITIK",
            Severity::Uyari => "UYARI",
            Severity::Bilgi => "BILGI",
            Severity::Iyi => "IYI",
        }
    }
}

struct Finding {
    sev: Severity,
    title: String,
    detail: String,
}

const HEAVY_MODES: [&str; 3] = ["hafif oyun", "AAA oyun", "render"];

fn analyze(s: &Session) -> Vec<Finding> {
    let mut out = Vec::new();

    // Driver / service warnings surfaced verbatim - these are the daemon
    // telling us something failed, not a heuristic guess.
    for w in &s.warnings {
        out.push(Finding {
            sev: Severity::Kritik,
            title: "Daemon uyarisi".into(),
            detail: w.clone(),
        });
    }

    // Ceiling throttled well below the mode's own max, with no thermal reason.
    // Only checked in modes where demand-based scaling is a bug, not a
    // feature - Idle/Office/Media are supposed to scale down.
    if !s.samples.is_empty() && !s.changes.is_empty() {
        let ceiling_for = |t: f32| -> Option<&ModeChange> {
            s.changes.iter().rev().find(|c| c.t <= t)
        };
        let mut checked = 0u32;
        let mut throttled = 0u32;
        for row in &s.samples {
            if !HEAVY_MODES.contains(&row.mode.as_str()) {
                continue;
            }
            let Some(c) = ceiling_for(row.t) else { continue };
            if c.mode != row.mode {
                continue;
            }
            checked += 1;
            let far_below_ceiling = (row.ceiling as f32) < (c.gpu_ceiling as f32) * 0.85;
            let no_heat_pressure = row.gpu_c < (c.gpu_target as f32) - 8.0;
            if far_below_ceiling && no_heat_pressure {
                throttled += 1;
            }
        }
        if checked >= 5 && throttled * 100 / checked >= 20 {
            out.push(Finding {
                sev: Severity::Uyari,
                title: "GPU frekansi isi gerekcesi olmadan kisilmis".into(),
                detail: format!(
                    "{checked} oyun ornekleminden {throttled} tanesinde ({}%) tavan, hedef sicakligin \
                     8C+ altindayken maksimumun %85'inden dusuktu. Bu 'talep kismasi' (demand_scaling) \
                     GPU'yu mesguliyete gore kisiyor olabilir - kare hizi kaybediyor olabilirsin.",
                    throttled * 100 / checked
                ),
            });
        }
    }

    // Mode flapping: same mode reappearing within two hops and under two
    // minutes means a threshold sits on top of the real workload. Idle/Office
    // flapping while sitting at the desktop is cheap and normal (near-zero fan,
    // near-zero clock) - only flag flapping that involves a game/render mode,
    // since that's the case that repeatedly rewrites fan curves and GPU clock
    // ceilings under real load.
    if s.changes.len() >= 3 {
        let mut flips = 0u32;
        for i in 2..s.changes.len() {
            let a = &s.changes[i];
            let b = &s.changes[i - 2];
            let involves_heavy =
                HEAVY_MODES.contains(&a.mode.as_str()) || HEAVY_MODES.contains(&b.mode.as_str());
            if a.mode == b.mode && (a.t - b.t) < 120.0 && involves_heavy {
                flips += 1;
            }
        }
        if flips >= 3 {
            out.push(Finding {
                sev: Severity::Uyari,
                title: "Mod salinimi".into(),
                detail: format!(
                    "{} mod degisiminden {flips} tanesi, iki dakikadan kisa surede oyun/render moduna \
                     geri donen salinim. Her seferinde fan egrisi ve GPU tavani gercek yuk altinda \
                     yeniden yaziliyor demektir.",
                    s.changes.len()
                ),
            });
        }
    }

    // RAM reclaim: fruitless passes only ever appear in log entries written by
    // the pre-fix binary (current code never logs a +0 MB pass). Their
    // presence here means this session predates the backoff fix.
    let fruitless: Vec<&ReclaimEvent> = s.reclaims.iter().filter(|r| r.freed_mb < 32).collect();
    if !fruitless.is_empty() {
        out.push(Finding {
            sev: Severity::Bilgi,
            title: "Eski surumden bosa giden RAM denemeleri".into(),
            detail: format!(
                "{} temizlik gecisi 32 MB'tan az kazandirmis. Guncel surum bunlari geri kazanim \
                 dongusune girmeden atlar (exponential backoff) - bu, guncelleme oncesi kaydedilmis \
                 bir oturum.",
                fruitless.len()
            ),
        });
    }
    let useful: Vec<&ReclaimEvent> = s.reclaims.iter().filter(|r| r.freed_mb >= 32).collect();
    if !useful.is_empty() {
        let total: i64 = useful.iter().map(|r| r.freed_mb).sum();
        out.push(Finding {
            sev: Severity::Iyi,
            title: "RAM geri kazanimi calisiyor".into(),
            detail: format!(
                "{} basarili gecis, toplam {total} MB gercekten bos belleğe donduruldu.",
                useful.len()
            ),
        });
    }

    // Thermal peaks. CPU has no software lever (locked PPT), so this is
    // informational, not actionable - but worth surfacing plainly.
    if let Some(max_cpu) = s.samples.iter().map(|r| r.cpu_c).fold(None, |a: Option<f32>, b| {
        Some(a.map_or(b, |a| a.max(b)))
    }) {
        if max_cpu >= 95.0 {
            out.push(Finding {
                sev: Severity::Bilgi,
                title: "CPU sicaklik tavanina yakin".into(),
                detail: format!(
                    "Bu oturumda CPU {max_cpu:.0}C'ye ulasti. Bu makinede CPU guc limiti firmware \
                     tarafindan kilitli - elimizdeki tek kol fan egrisi, o da zaten tavanda."
                ),
            });
        }
    }
    if let Some(max_gpu) = s.samples.iter().map(|r| r.gpu_c).fold(None, |a: Option<f32>, b| {
        Some(a.map_or(b, |a| a.max(b)))
    }) {
        let target = s
            .changes
            .iter()
            .filter(|c| HEAVY_MODES.contains(&c.mode.as_str()))
            .map(|c| c.gpu_target)
            .max()
            .unwrap_or(80);
        if max_gpu <= target as f32 + 1.0 {
            out.push(Finding {
                sev: Severity::Iyi,
                title: "GPU sicakligi hedefte tutuldu".into(),
                detail: format!(
                    "Tepe GPU sicakligi {max_gpu:.0}C, hedef {target}C'nin uzerine cikmadi - governor \
                     (termal kirpma) beklendigi gibi calisiyor."
                ),
            });
        } else {
            out.push(Finding {
                sev: Severity::Uyari,
                title: "GPU hedefin uzerine cikti".into(),
                detail: format!("Tepe GPU sicakligi {max_gpu:.0}C, hedef {target}C'yi asti."),
            });
        }
    }

    if !s.ended_clean {
        out.push(Finding {
            sev: Severity::Bilgi,
            title: "Oturum temiz kapanmadi".into(),
            detail: "Log'da bu oturum icin bir sonlanma satiri yok - daemon yeniden baslatildi, \
                      cokme, ya da makine kapatildi."
                .into(),
        });
    }

    out.sort_by_key(|f| match f.sev {
        Severity::Kritik => 0,
        Severity::Uyari => 1,
        Severity::Bilgi => 2,
        Severity::Iyi => 3,
    });
    out
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn mode_color(mode: &str) -> &'static str {
    match mode {
        "bosta" => "#6b7280",
        "ofis" => "#3b82f6",
        "film" => "#8b5cf6",
        "hafif oyun" => "#14b8a6",
        "AAA oyun" => "#f97316",
        "render" => "#ef4444",
        _ => "#6b7280",
    }
}

fn avg(v: impl Iterator<Item = f32> + Clone) -> f32 {
    let mut n = 0u32;
    let mut sum = 0.0f32;
    for x in v {
        sum += x;
        n += 1;
    }
    if n == 0 {
        0.0
    } else {
        sum / n as f32
    }
}

/// Stacked-band + dual-line SVG chart, built directly from samples - no JS
/// charting library, so the file still renders after being copied anywhere.
fn render_chart(s: &Session) -> String {
    if s.samples.is_empty() {
        return "<p class=\"muted\">Bu oturumda grafik icin yeterli ornek yok.</p>".to_string();
    }
    let w = 900.0f32;
    let h = 260.0f32;
    let pad_l = 40.0f32;
    let pad_b = 24.0f32;
    let plot_w = w - pad_l - 10.0;
    let plot_h = h - pad_b - 10.0;
    let t_max = s.samples.last().unwrap().t.max(1.0);

    let x = |t: f32| pad_l + (t / t_max) * plot_w;
    let y = |c: f32| 10.0 + plot_h - (c.clamp(0.0, 100.0) / 100.0) * plot_h;

    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg viewBox=\"0 0 {w} {h}\" width=\"100%\" height=\"auto\" role=\"img\">"
    ));

    // Mode bands.
    for (i, c) in s.changes.iter().enumerate() {
        let t0 = c.t;
        let t1 = s.changes.get(i + 1).map(|n| n.t).unwrap_or(t_max);
        svg.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"10\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{}\" opacity=\"0.12\"/>",
            x(t0),
            (x(t1) - x(t0)).max(0.0),
            plot_h,
            mode_color(&c.mode)
        ));
    }

    // Grid + axis labels at 20/40/60/80/100C.
    for gc in [20, 40, 60, 80, 100] {
        let gy = y(gc as f32);
        svg.push_str(&format!(
            "<line x1=\"{pad_l}\" y1=\"{gy:.1}\" x2=\"{:.1}\" y2=\"{gy:.1}\" stroke=\"var(--grid)\" stroke-width=\"1\"/>\
             <text x=\"4\" y=\"{:.1}\" fill=\"var(--muted)\" font-size=\"11\">{gc}C</text>",
            pad_l + plot_w,
            gy + 3.0
        ));
    }

    let poly = |get: fn(&SampleRow) -> f32| -> String {
        s.samples
            .iter()
            .map(|r| format!("{:.1},{:.1}", x(r.t), y(get(r))))
            .collect::<Vec<_>>()
            .join(" ")
    };
    svg.push_str(&format!(
        "<polyline points=\"{}\" fill=\"none\" stroke=\"#60a5fa\" stroke-width=\"2\"/>",
        poly(|r| r.gpu_c)
    ));
    svg.push_str(&format!(
        "<polyline points=\"{}\" fill=\"none\" stroke=\"#f87171\" stroke-width=\"2\"/>",
        poly(|r| r.cpu_c)
    ));
    // Fan RPM has no natural 0-100 scale, so it rides the same axis divided by
    // 70 (max useful fan speed on this hardware is ~7000 RPM) - a dashed line
    // makes clear it is not a temperature.
    svg.push_str(&format!(
        "<polyline points=\"{}\" fill=\"none\" stroke=\"var(--muted)\" stroke-width=\"1.5\" stroke-dasharray=\"4 3\"/>",
        poly(|r| r.fan_rpm as f32 / 70.0)
    ));

    // Time axis ticks, every ~20% of the session.
    for frac in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let t = t_max * frac;
        svg.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{}\" fill=\"var(--muted)\" font-size=\"11\" text-anchor=\"middle\">{}dk</text>",
            x(t),
            h - 6.0,
            (t / 60.0).round() as i64
        ));
    }

    svg.push_str("</svg>");
    svg
}

fn render_timeline(s: &Session) -> String {
    if s.changes.is_empty() {
        return String::new();
    }
    let t_max = s.samples.last().map(|r| r.t).unwrap_or(s.changes.last().unwrap().t).max(1.0);
    let mut segs = String::new();
    for (i, c) in s.changes.iter().enumerate() {
        let t0 = c.t;
        let t1 = s.changes.get(i + 1).map(|n| n.t).unwrap_or(t_max);
        let pct0 = t0 / t_max * 100.0;
        let pct1 = ((t1 - t0) / t_max * 100.0).max(0.3);
        segs.push_str(&format!(
            "<div class=\"seg\" style=\"left:{pct0:.2}%;width:{pct1:.2}%;background:{}\" title=\"{} @ {:.0}s\"></div>",
            mode_color(&c.mode),
            esc(&c.mode),
            t0
        ));
    }
    format!("<div class=\"timeline\">{segs}</div>")
}

fn render_legend() -> String {
    let mut out = String::from("<div class=\"legend\">");
    for m in KNOWN_MODES {
        out.push_str(&format!(
            "<span class=\"legend-item\"><i style=\"background:{}\"></i>{}</span>",
            mode_color(m),
            esc(m)
        ));
    }
    out.push_str("<span class=\"legend-item\"><i style=\"background:#f87171\"></i>CPU sicaklik</span>");
    out.push_str("<span class=\"legend-item\"><i style=\"background:#60a5fa\"></i>GPU sicaklik</span>");
    out.push_str("<span class=\"legend-item\"><i style=\"background:var(--muted)\"></i>fan hizi (kesikli, /70 RPM)</span>");
    out.push_str("</div>");
    out
}

fn render_session(idx: usize, s: &Session, active: bool) -> String {
    let dur_s = s.samples.last().map(|r| r.t).unwrap_or(0.0);
    let max_cpu = s.samples.iter().map(|r| r.cpu_c).fold(0.0f32, f32::max);
    let max_gpu = s.samples.iter().map(|r| r.gpu_c).fold(0.0f32, f32::max);
    let avg_cpu = avg(s.samples.iter().map(|r| r.cpu_c));
    let avg_gpu = avg(s.samples.iter().map(|r| r.gpu_c));
    let avg_cpu_pct = avg(s.samples.iter().map(|r| r.cpu_pct));
    let avg_gpu_pct = avg(s.samples.iter().map(|r| r.gpu_pct));
    let avg_fan = avg(s.samples.iter().map(|r| r.fan_rpm as f32));
    let total_freed: i64 = s.reclaims.iter().filter(|r| r.freed_mb > 0).map(|r| r.freed_mb).sum();
    let profiles: Vec<&str> = {
        let mut v: Vec<&str> = s
            .changes
            .iter()
            .filter_map(|c| c.profile.as_deref())
            .collect();
        v.dedup();
        v
    };

    let findings = analyze(s);
    let findings_html: String = if findings.is_empty() {
        "<p class=\"muted\">Bulgu yok.</p>".to_string()
    } else {
        findings
            .iter()
            .map(|f| {
                format!(
                    "<div class=\"finding {}\"><span class=\"badge\">{}</span><div><b>{}</b><p>{}</p></div></div>",
                    f.sev.tag(),
                    f.sev.label(),
                    esc(&f.title),
                    esc(&f.detail)
                )
            })
            .collect()
    };

    let changes_rows: String = s
        .changes
        .iter()
        .map(|c| {
            format!(
                "<tr><td>{:.0}s</td><td>{}</td><td>{}</td><td>{}</td><td>%{}-{}</td><td>{}-{}MHz</td><td>{}C / {}C</td></tr>",
                c.t,
                esc(&c.mode),
                c.profile.as_deref().map(esc).unwrap_or_default(),
                esc(&c.power),
                c.fan_idle,
                c.fan_max,
                c.gpu_floor,
                c.gpu_ceiling,
                c.cpu_target,
                c.gpu_target
            )
        })
        .collect();

    let reclaim_rows: String = s
        .reclaims
        .iter()
        .map(|r| {
            format!(
                "<tr><td>{:.0}s</td><td>{}</td><td>{}</td><td>{}{} MB</td></tr>",
                r.t,
                esc(&r.trigger),
                esc(&r.depth),
                if r.freed_mb >= 0 { "+" } else { "" },
                r.freed_mb
            )
        })
        .collect();

    format!(
        r#"<section class="session {vis}" id="s{idx}">
  {forced_badge}
  <div class="cards">
    <div class="card"><span class="k">sure</span><span class="v">{dur_m} dk</span></div>
    <div class="card"><span class="k">CPU tepe / ort</span><span class="v">{max_cpu:.0}C / {avg_cpu:.0}C</span></div>
    <div class="card"><span class="k">GPU tepe / ort</span><span class="v">{max_gpu:.0}C / {avg_gpu:.0}C</span></div>
    <div class="card"><span class="k">ort yuk (CPU / GPU)</span><span class="v">%{avg_cpu_pct:.0} / %{avg_gpu_pct:.0}</span></div>
    <div class="card"><span class="k">ort fan</span><span class="v">{avg_fan:.0} RPM</span></div>
    <div class="card"><span class="k">mod degisimi</span><span class="v">{changes}</span></div>
    <div class="card"><span class="k">RAM geri kazanildi</span><span class="v">{total_freed} MB</span></div>
    <div class="card"><span class="k">oyun profili</span><span class="v">{profile_txt}</span></div>
  </div>

  {legend}
  <div class="chart-wrap">{chart}</div>
  {timeline}

  <h3>Bulgular</h3>
  <div class="findings">{findings_html}</div>

  <details>
    <summary>Mod degisim gunlugu ({changes} kayit)</summary>
    <table>
      <thead><tr><th>t</th><th>mod</th><th>profil</th><th>guc</th><th>fan</th><th>GPU</th><th>hedef</th></tr></thead>
      <tbody>{changes_rows}</tbody>
    </table>
  </details>

  <details>
    <summary>RAM geri kazanim olaylari ({reclaim_n} kayit)</summary>
    <table>
      <thead><tr><th>t</th><th>tetikleyici</th><th>derinlik</th><th>kazanc</th></tr></thead>
      <tbody>{reclaim_rows}</tbody>
    </table>
  </details>
</section>"#,
        vis = if active { "active" } else { "" },
        forced_badge = if s.forced {
            "<p class=\"muted\">Bu oturum otomatik siniflandirmayi degil, dayatilmis (force) bir modu kullandi.</p>".to_string()
        } else {
            String::new()
        },
        dur_m = (dur_s / 60.0).round() as i64,
        changes = s.changes.len(),
        reclaim_n = s.reclaims.len(),
        profile_txt = if profiles.is_empty() { "-".to_string() } else { esc(&profiles.join(", ")) },
        legend = render_legend(),
        chart = render_chart(s),
        timeline = render_timeline(s),
    )
}

fn render_html(sessions: &[Session]) -> String {
    let tabs: String = sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            // Chronological order: oldest is Oturum 1, most recent has the
            // highest number and is the one shown by default.
            let n = i + 1;
            let dur_m = (s.samples.last().map(|r| r.t).unwrap_or(0.0) / 60.0).round() as i64;
            let is_last = i == sessions.len() - 1;
            let tag = if is_last { " (guncel)" } else { "" };
            format!(
                "<button class=\"tab{}\" onclick=\"showSession({i})\">Oturum {n}{tag} <small>({dur_m} dk)</small></button>",
                if is_last { " active" } else { "" }
            )
        })
        .collect();

    let body: String = sessions
        .iter()
        .enumerate()
        .map(|(i, s)| render_session(i, s, i == sessions.len() - 1))
        .collect();

    format!(
        r#"<!doctype html>
<html lang="tr"><head><meta charset="utf-8"><title>rogctl raporu</title>
<style>
:root {{
  --bg:#0f1115; --panel:#171a21; --text:#e6e9ef; --muted:#8b93a3; --grid:#2a2f3a;
  --border:#262b35; --kritik:#ef4444; --uyari:#f59e0b; --bilgi:#3b82f6; --iyi:#22c55e;
}}
@media (prefers-color-scheme: light) {{
  :root:not([data-theme="dark"]) {{
    --bg:#f5f6f8; --panel:#ffffff; --text:#1a1d24; --muted:#5b6270; --grid:#e2e5eb; --border:#e2e5eb;
  }}
}}
* {{ box-sizing:border-box; }}
body {{ margin:0; background:var(--bg); color:var(--text); font:14px/1.5 -apple-system,Segoe UI,sans-serif; }}
header {{ padding:20px 24px 0; }}
h1 {{ font-size:20px; margin:0 0 4px; }}
.sub {{ color:var(--muted); font-size:13px; margin-bottom:16px; }}
.tabs {{ display:flex; gap:8px; padding:0 24px; border-bottom:1px solid var(--border); flex-wrap:wrap; }}
.tab {{ background:none; border:none; color:var(--muted); padding:10px 14px; cursor:pointer; font-size:13px; border-bottom:2px solid transparent; }}
.tab.active {{ color:var(--text); border-bottom-color:#60a5fa; }}
.tab small {{ color:var(--muted); }}
main {{ padding:20px 24px 40px; }}
.session {{ display:none; }}
.session.active {{ display:block; }}
.cards {{ display:grid; grid-template-columns:repeat(auto-fit,minmax(140px,1fr)); gap:10px; margin-bottom:16px; }}
.card {{ background:var(--panel); border:1px solid var(--border); border-radius:10px; padding:12px 14px; display:flex; flex-direction:column; gap:4px; }}
.card .k {{ color:var(--muted); font-size:11px; text-transform:uppercase; letter-spacing:.04em; }}
.card .v {{ font-size:18px; font-weight:600; }}
.legend {{ display:flex; flex-wrap:wrap; gap:12px; font-size:12px; color:var(--muted); margin-bottom:8px; }}
.legend-item {{ display:flex; align-items:center; gap:5px; }}
.legend-item i {{ width:10px; height:10px; border-radius:2px; display:inline-block; }}
.chart-wrap {{ background:var(--panel); border:1px solid var(--border); border-radius:10px; padding:10px; }}
.timeline {{ position:relative; height:14px; margin:10px 0 20px; border-radius:4px; overflow:hidden; background:var(--grid); }}
.seg {{ position:absolute; top:0; bottom:0; }}
h3 {{ font-size:14px; margin:20px 0 8px; }}
.findings {{ display:flex; flex-direction:column; gap:8px; }}
.finding {{ display:flex; gap:10px; background:var(--panel); border:1px solid var(--border); border-left:4px solid var(--muted); border-radius:8px; padding:10px 12px; }}
.finding.kritik {{ border-left-color:var(--kritik); }}
.finding.uyari {{ border-left-color:var(--uyari); }}
.finding.bilgi {{ border-left-color:var(--bilgi); }}
.finding.iyi {{ border-left-color:var(--iyi); }}
.finding p {{ margin:4px 0 0; color:var(--muted); }}
.badge {{ font-size:10px; font-weight:700; letter-spacing:.05em; color:var(--muted); white-space:nowrap; padding-top:2px; }}
.finding.kritik .badge {{ color:var(--kritik); }}
.finding.uyari .badge {{ color:var(--uyari); }}
.finding.bilgi .badge {{ color:var(--bilgi); }}
.finding.iyi .badge {{ color:var(--iyi); }}
.muted {{ color:var(--muted); }}
details {{ margin-top:8px; }}
summary {{ cursor:pointer; color:var(--muted); }}
table {{ width:100%; border-collapse:collapse; margin-top:10px; font-size:12px; }}
th,td {{ text-align:left; padding:6px 8px; border-bottom:1px solid var(--border); }}
th {{ color:var(--muted); font-weight:600; }}
</style></head>
<body>
<header>
  <h1>rogctl raporu</h1>
  <div class="sub">bin\rogctl.log analizi &middot; en yeni oturum varsayilan olarak acik</div>
</header>
<div class="tabs">{tabs}</div>
<main>{body}</main>
<script>
function showSession(i) {{
  document.querySelectorAll('.session').forEach((el,idx)=>el.classList.toggle('active', idx===i));
  document.querySelectorAll('.tab').forEach((el,idx)=>el.classList.toggle('active', idx===i));
}}
</script>
</body></html>"#
    )
}

/// Parse the log and write the HTML report next to it. Returns the output path.
pub fn run(log_path: PathBuf, out_path: PathBuf) -> anyhow::Result<PathBuf> {
    let text = std::fs::read_to_string(&log_path)
        .map_err(|e| anyhow::anyhow!("log okunamadi ({}): {e}", log_path.display()))?;
    let sessions = parse_log(&text);
    if sessions.is_empty() {
        anyhow::bail!("log'da taninabilir bir oturum bulunamadi");
    }
    let html = render_html(&sessions);
    std::fs::write(&out_path, html)?;
    Ok(out_path)
}
