//! Session export — export terminal sessions as asciinema, text, HTML, or JSON.

use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct SessionFrame {
    pub timestamp: f64,     // Seconds since session start
    pub event_type: String, // "o" for output, "i" for input
    pub data: String,
}

#[derive(Debug, Clone)]
pub struct SessionRecording {
    pub version: u8,
    pub width: u32,
    pub height: u32,
    pub title: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub frames: Vec<SessionFrame>,
}

/// Escape a string for embedding in JSON.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

impl SessionRecording {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            version: 2,
            width,
            height,
            title: Some("Volt Terminal Session".to_string()),
            env: None,
            frames: Vec::new(),
        }
    }

    pub fn add_output(&mut self, timestamp: f64, data: String) {
        self.frames.push(SessionFrame {
            timestamp,
            event_type: "o".to_string(),
            data,
        });
    }

    pub fn add_input(&mut self, timestamp: f64, data: String) {
        self.frames.push(SessionFrame {
            timestamp,
            event_type: "i".to_string(),
            data,
        });
    }

    pub fn duration(&self) -> f64 {
        self.frames.last().map(|f| f.timestamp).unwrap_or(0.0)
    }

    /// Export as asciinema v2 format (.cast)
    pub fn to_asciinema(&self) -> String {
        let mut lines = Vec::new();

        // Header line (JSON object)
        let title_json = match &self.title {
            Some(t) => format!("\"{}\"", json_escape(t)),
            None => "null".to_string(),
        };
        lines.push(format!(
            r#"{{"version": {}, "width": {}, "height": {}, "title": {}}}"#,
            self.version, self.width, self.height, title_json
        ));

        // Frame lines: [timestamp, event_type, data]
        for frame in &self.frames {
            let escaped = json_escape(&frame.data);
            lines.push(format!(
                r#"[{:.6}, "{}", "{}"]"#,
                frame.timestamp, frame.event_type, escaped
            ));
        }

        lines.join("\n") + "\n"
    }

    /// Export as plain text (output only)
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        for frame in &self.frames {
            if frame.event_type == "o" {
                output.push_str(&frame.data);
            }
        }
        output
    }

    /// Export as a simple HTML page with an asciinema player
    pub fn to_html(&self) -> String {
        let asciinema_data = self
            .to_asciinema()
            .replace('\\', "\\\\")
            .replace('`', "\\`")
            .replace("${", "\\${");

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>{title}</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/asciinema-player@3.7.0/dist/bundle/asciinema-player.min.css">
    <style>
        body {{ background: #1e1e2e; margin: 0; display: flex; justify-content: center; align-items: center; min-height: 100vh; }}
        #player {{ max-width: 900px; width: 100%; }}
        h1 {{ color: #f5c211; text-align: center; font-family: system-ui; margin: 20px 0; }}
    </style>
</head>
<body>
    <div>
        <h1>{title}</h1>
        <div id="player"></div>
    </div>
    <script src="https://cdn.jsdelivr.net/npm/asciinema-player@3.7.0/dist/bundle/asciinema-player.min.js"></script>
    <script>
        const castData = `{data}`;
        const blob = new Blob([castData], {{ type: 'text/plain' }});
        const url = URL.createObjectURL(blob);
        AsciinemaPlayer.create(url, document.getElementById('player'), {{
            theme: 'monokai',
            fit: 'width',
            autoPlay: true,
        }});
    </script>
</body>
</html>"#,
            title = self.title.as_deref().unwrap_or("Volt Session"),
            data = asciinema_data,
        )
    }

    /// Export as pretty-printed JSON
    pub fn to_json(&self) -> String {
        let title_json = match &self.title {
            Some(t) => format!("\"{}\"", json_escape(t)),
            None => "null".to_string(),
        };

        let env_json = match &self.env {
            Some(map) => {
                let entries: Vec<String> = map
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "      \"{}\": \"{}\"",
                            json_escape(k),
                            json_escape(v)
                        )
                    })
                    .collect();
                format!("{{\n{}\n    }}", entries.join(",\n"))
            }
            None => "null".to_string(),
        };

        let frames_json: Vec<String> = self
            .frames
            .iter()
            .map(|f| {
                format!(
                    "    {{\n      \"timestamp\": {},\n      \"event_type\": \"{}\",\n      \"data\": \"{}\"\n    }}",
                    f.timestamp,
                    json_escape(&f.event_type),
                    json_escape(&f.data)
                )
            })
            .collect();

        format!(
            "{{\n  \"version\": {},\n  \"width\": {},\n  \"height\": {},\n  \"title\": {},\n  \"env\": {},\n  \"frames\": [\n{}\n  ]\n}}\n",
            self.version,
            self.width,
            self.height,
            title_json,
            env_json,
            frames_json.join(",\n")
        )
    }

    /// Save to file with appropriate format based on extension
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("cast");

        let content = match ext {
            "cast" => self.to_asciinema(),
            "txt" => self.to_text(),
            "html" => self.to_html(),
            "json" => self.to_json(),
            _ => return Err(format!("Unsupported format: .{}", ext)),
        };

        std::fs::write(path, content).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_recording() -> SessionRecording {
        let mut rec = SessionRecording::new(80, 24);
        rec.add_output(0.0, "$ ".to_string());
        rec.add_input(0.5, "ls\r".to_string());
        rec.add_output(0.6, "file1.txt  file2.txt\r\n$ ".to_string());
        rec
    }

    #[test]
    fn test_asciinema_export() {
        let rec = sample_recording();
        let cast = rec.to_asciinema();
        assert!(cast.contains("\"version\": 2"));
        assert!(cast.contains("\"width\": 80"));
        assert!(cast.lines().count() >= 4); // header + 3 frames
    }

    #[test]
    fn test_text_export() {
        let rec = sample_recording();
        let text = rec.to_text();
        assert!(text.contains("$ "));
        assert!(text.contains("file1.txt"));
        // Should not contain input
        assert!(!text.contains("ls\r"));
    }

    #[test]
    fn test_html_export() {
        let rec = sample_recording();
        let html = rec.to_html();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("asciinema-player"));
        assert!(html.contains("Volt"));
    }

    #[test]
    fn test_json_export() {
        let rec = sample_recording();
        let json = rec.to_json();
        assert!(json.contains("\"version\": 2"));
        assert!(json.contains("\"frames\""));
    }

    #[test]
    fn test_duration() {
        let rec = sample_recording();
        assert!((rec.duration() - 0.6).abs() < 0.001);
    }
}
