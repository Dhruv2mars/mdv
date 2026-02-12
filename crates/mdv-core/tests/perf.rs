use std::time::Instant;

use mdv_core::render_preview_lines;

fn p95_us(values: &[u128]) -> u128 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let idx = ((sorted.len() as f64) * 0.95).ceil() as usize - 1;
    sorted[idx]
}

#[test]
#[ignore = "perf smoke runs in perf CI job"]
fn render_preview_lines_p95_under_budget() {
    let mut markdown = String::new();
    while markdown.len() < 200 * 1024 {
        markdown.push_str("# head\n- one\n- two\n`inline`\n\n");
    }

    let mut samples = Vec::new();
    for _ in 0..50 {
        let started = Instant::now();
        let _ = render_preview_lines(&markdown, 100);
        samples.push(started.elapsed().as_micros());
    }

    let p95 = p95_us(&samples);
    assert!(p95 <= 20_000, "p95_us={p95}");
}
