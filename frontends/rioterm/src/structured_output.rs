/// Structured output detection and rendering data model.
/// Detects JSON, tables, CSV/TSV, and key-value output formats.

#[derive(Debug, Clone)]
pub enum StructuredData {
    Json(JsonNode),
    Table(TableData),
    KeyValue(Vec<(String, String)>),
}

#[derive(Debug, Clone)]
pub enum JsonNode {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<JsonNode>),
    Object(Vec<(String, JsonNode)>),
}

#[derive(Debug, Clone)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub sort_column: Option<usize>,
    pub sort_ascending: bool,
}

impl TableData {
    pub fn sort_by_column(&mut self, col: usize) {
        if col >= self.headers.len() {
            return;
        }
        if self.sort_column == Some(col) {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = Some(col);
            self.sort_ascending = true;
        }
        let asc = self.sort_ascending;
        self.rows.sort_by(|a, b| {
            let va = a.get(col).map(|s| s.as_str()).unwrap_or("");
            let vb = b.get(col).map(|s| s.as_str()).unwrap_or("");
            // Try numeric comparison first
            if let (Ok(na), Ok(nb)) = (va.parse::<f64>(), vb.parse::<f64>()) {
                let cmp = na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal);
                return if asc { cmp } else { cmp.reverse() };
            }
            let cmp = va.cmp(vb);
            if asc {
                cmp
            } else {
                cmp.reverse()
            }
        });
    }
}

/// Try to detect structured output from raw text.
/// Returns None if the output is unstructured.
pub fn detect_structured(output: &str) -> Option<StructuredData> {
    let trimmed = output.trim();
    if trimmed.is_empty() || trimmed.len() > 1_000_000 {
        return None; // Too empty or too large
    }

    // Try JSON first (starts with { or [)
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if let Some(json) = try_parse_json(trimmed) {
            return Some(StructuredData::Json(json));
        }
    }

    // Try key-value detection (before table, since "key: value" lines
    // can be misdetected as whitespace-separated table columns)
    if let Some(kv) = try_detect_key_value(trimmed) {
        return Some(StructuredData::KeyValue(kv));
    }

    // Try table detection (aligned columns)
    if let Some(table) = try_detect_table(trimmed) {
        return Some(StructuredData::Table(table));
    }

    None
}

fn try_parse_json(input: &str) -> Option<JsonNode> {
    // Simple recursive descent JSON parser (no external deps)
    let bytes = input.as_bytes();
    let (node, _) = parse_json_value(bytes, 0)?;
    Some(node)
}

fn skip_whitespace(bytes: &[u8], mut pos: usize) -> usize {
    while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    pos
}

fn parse_json_value(bytes: &[u8], pos: usize) -> Option<(JsonNode, usize)> {
    let pos = skip_whitespace(bytes, pos);
    if pos >= bytes.len() {
        return None;
    }
    match bytes[pos] {
        b'"' => parse_json_string(bytes, pos),
        b'{' => parse_json_object(bytes, pos),
        b'[' => parse_json_array(bytes, pos),
        b't' | b'f' => parse_json_bool(bytes, pos),
        b'n' => parse_json_null(bytes, pos),
        b'-' | b'0'..=b'9' => parse_json_number(bytes, pos),
        _ => None,
    }
}

fn parse_json_string(bytes: &[u8], pos: usize) -> Option<(JsonNode, usize)> {
    if bytes[pos] != b'"' {
        return None;
    }
    let mut i = pos + 1;
    let mut s = String::new();
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'"' => {
                    s.push('"');
                    i += 2;
                }
                b'\\' => {
                    s.push('\\');
                    i += 2;
                }
                b'n' => {
                    s.push('\n');
                    i += 2;
                }
                b't' => {
                    s.push('\t');
                    i += 2;
                }
                b'r' => {
                    s.push('\r');
                    i += 2;
                }
                _ => {
                    s.push(bytes[i + 1] as char);
                    i += 2;
                }
            }
        } else if bytes[i] == b'"' {
            return Some((JsonNode::String(s), i + 1));
        } else {
            s.push(bytes[i] as char);
            i += 1;
        }
    }
    None
}

fn parse_json_object(bytes: &[u8], pos: usize) -> Option<(JsonNode, usize)> {
    if bytes[pos] != b'{' {
        return None;
    }
    let mut i = skip_whitespace(bytes, pos + 1);
    let mut entries = Vec::new();
    if i < bytes.len() && bytes[i] == b'}' {
        return Some((JsonNode::Object(entries), i + 1));
    }
    loop {
        i = skip_whitespace(bytes, i);
        let (key_node, next) = parse_json_string(bytes, i)?;
        let key = match key_node {
            JsonNode::String(s) => s,
            _ => return None,
        };
        i = skip_whitespace(bytes, next);
        if i >= bytes.len() || bytes[i] != b':' {
            return None;
        }
        i = skip_whitespace(bytes, i + 1);
        let (val, next) = parse_json_value(bytes, i)?;
        entries.push((key, val));
        i = skip_whitespace(bytes, next);
        if i >= bytes.len() {
            return None;
        }
        if bytes[i] == b'}' {
            return Some((JsonNode::Object(entries), i + 1));
        }
        if bytes[i] != b',' {
            return None;
        }
        i += 1;
    }
}

fn parse_json_array(bytes: &[u8], pos: usize) -> Option<(JsonNode, usize)> {
    if bytes[pos] != b'[' {
        return None;
    }
    let mut i = skip_whitespace(bytes, pos + 1);
    let mut items = Vec::new();
    if i < bytes.len() && bytes[i] == b']' {
        return Some((JsonNode::Array(items), i + 1));
    }
    loop {
        i = skip_whitespace(bytes, i);
        let (val, next) = parse_json_value(bytes, i)?;
        items.push(val);
        i = skip_whitespace(bytes, next);
        if i >= bytes.len() {
            return None;
        }
        if bytes[i] == b']' {
            return Some((JsonNode::Array(items), i + 1));
        }
        if bytes[i] != b',' {
            return None;
        }
        i += 1;
    }
}

fn parse_json_bool(bytes: &[u8], pos: usize) -> Option<(JsonNode, usize)> {
    if bytes[pos..].starts_with(b"true") {
        return Some((JsonNode::Bool(true), pos + 4));
    }
    if bytes[pos..].starts_with(b"false") {
        return Some((JsonNode::Bool(false), pos + 5));
    }
    None
}

fn parse_json_null(bytes: &[u8], pos: usize) -> Option<(JsonNode, usize)> {
    if bytes[pos..].starts_with(b"null") {
        return Some((JsonNode::Null, pos + 4));
    }
    None
}

fn parse_json_number(bytes: &[u8], pos: usize) -> Option<(JsonNode, usize)> {
    let mut i = pos;
    if i < bytes.len() && bytes[i] == b'-' {
        i += 1;
    }
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return None;
    }
    while i < bytes.len()
        && (bytes[i].is_ascii_digit()
            || bytes[i] == b'.'
            || bytes[i] == b'e'
            || bytes[i] == b'E'
            || bytes[i] == b'+'
            || bytes[i] == b'-')
    {
        i += 1;
    }
    let s = std::str::from_utf8(&bytes[pos..i]).ok()?;
    Some((JsonNode::Number(s.to_string()), i))
}

fn try_detect_table(input: &str) -> Option<TableData> {
    let lines: Vec<&str> = input.lines().collect();
    if lines.len() < 2 {
        return None;
    }

    // Check if first line looks like headers (mostly uppercase or title case words)
    let header_line = lines[0];
    let headers: Vec<String> = header_line
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    if headers.len() < 2 {
        return None;
    }

    // Check if subsequent lines have similar column count
    let mut rows = Vec::new();
    let header_count = headers.len();
    let mut matching_lines = 0;

    // Skip separator lines (--- or ===)
    for line in &lines[1..] {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed
                .chars()
                .all(|c| c == '-' || c == '=' || c == '+' || c == '|')
        {
            continue;
        }
        let cols: Vec<String> =
            trimmed.split_whitespace().map(|s| s.to_string()).collect();
        // Allow +/-2 column variance (some rows may have spaces in values)
        if cols.len() >= header_count.saturating_sub(2) && cols.len() <= header_count + 2
        {
            // Pad or truncate to match header count
            let mut row = cols;
            row.resize(header_count, String::new());
            rows.push(row);
            matching_lines += 1;
        }
    }

    // Need at least 50% of data lines to match
    let data_lines = lines.len() - 1;
    if matching_lines < 1 || (data_lines > 3 && matching_lines * 2 < data_lines) {
        return None;
    }

    Some(TableData {
        headers,
        rows,
        sort_column: None,
        sort_ascending: true,
    })
}

fn try_detect_key_value(input: &str) -> Option<Vec<(String, String)>> {
    let lines: Vec<&str> = input.lines().collect();
    if lines.len() < 2 {
        return None;
    }

    let mut pairs = Vec::new();
    let mut kv_count = 0;

    for line in &lines {
        // Check for "key: value" or "key = value" pattern
        if let Some(sep_pos) = line.find(": ").or_else(|| line.find(" = ")) {
            let key = line[..sep_pos].trim().to_string();
            let sep_len = if line[sep_pos..].starts_with(": ") {
                2
            } else {
                3
            };
            let value = line[sep_pos + sep_len..].trim().to_string();
            if !key.is_empty() && !key.contains(' ') {
                pairs.push((key, value));
                kv_count += 1;
            }
        }
    }

    // Need at least 60% of lines to be key-value
    if kv_count < 2 || kv_count * 5 < lines.len() * 3 {
        return None;
    }

    Some(pairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_detection() {
        let json = r#"{"name": "volt", "version": 1, "features": ["tabs", "splits"]}"#;
        let result = detect_structured(json);
        assert!(matches!(result, Some(StructuredData::Json(_))));
    }

    #[test]
    fn test_table_detection() {
        let table = "NAME  STATUS  AGE\npod1  Running  5d\npod2  Pending  1h\n";
        let result = detect_structured(table);
        assert!(matches!(result, Some(StructuredData::Table(_))));
    }

    #[test]
    fn test_kv_detection() {
        let kv = "name: volt\nversion: 0.1.0\nlicense: MIT\n";
        let result = detect_structured(kv);
        assert!(matches!(result, Some(StructuredData::KeyValue(_))));
    }
}
