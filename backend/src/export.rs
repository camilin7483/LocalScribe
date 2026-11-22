use crate::domain::{Job, Segment};

pub fn stamp(ms: i64, separator: char) -> String {
    format!(
        "{:02}:{:02}:{:02}{separator}{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

fn caption(segment: &Segment) -> String {
    match &segment.speaker {
        Some(speaker) => format!("{speaker}: {}", segment.text),
        None => segment.text.clone(),
    }
}

fn cue_text(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn make(
    job: &Job,
    segments: &[Segment],
    format: &str,
) -> anyhow::Result<Option<(String, &'static str)>> {
    use std::fmt::Write;
    let mut output = String::new();
    let content_type = match format {
        "txt" => {
            for segment in segments {
                writeln!(output, "{}", caption(segment))?;
            }
            "text/plain; charset=utf-8"
        }
        "md" => {
            for segment in segments {
                let text = caption(segment)
                    .replace('\\', "\\\\")
                    .replace(['\r', '\n'], " ");
                let safe: String = text
                    .chars()
                    .flat_map(|character| {
                        ['<', '>', '[', ']', '*', '_', '`', '~', '#']
                            .contains(&character)
                            .then_some('\\')
                            .into_iter()
                            .chain(std::iter::once(character))
                    })
                    .collect();
                writeln!(output, "- **{}** {safe}", stamp(segment.start_ms, '.'))?;
            }
            "text/markdown; charset=utf-8"
        }
        "json" => {
            output =
                serde_json::to_string_pretty(&serde_json::json!({"job":job,"segments":segments}))?;
            "application/json"
        }
        "srt" | "vtt" => {
            let webvtt = format == "vtt";
            let separator = if webvtt { '.' } else { ',' };
            if webvtt {
                output.push_str("WEBVTT\n\n");
            }
            for (index, segment) in segments
                .iter()
                .filter(|segment| !segment.text.trim().is_empty())
                .enumerate()
            {
                let text = cue_text(&caption(segment));
                let text = if webvtt { escaped(&text) } else { text };
                writeln!(
                    output,
                    "{}\n{} --> {}\n{text}\n",
                    index + 1,
                    stamp(segment.start_ms, separator),
                    stamp(segment.end_ms, separator)
                )?;
            }
            if webvtt {
                "text/vtt; charset=utf-8"
            } else {
                "application/x-subrip; charset=utf-8"
            }
        }
        _ => return Ok(None),
    };
    Ok(Some((output, content_type)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stamps_long_recordings() {
        assert_eq!(stamp(3_661_007, ','), "01:01:01,007");
        assert_eq!(cue_text("uno\n\ndos"), "uno\ndos");
        assert_eq!(escaped("<i>&"), "&lt;i&gt;&amp;");
    }
}
