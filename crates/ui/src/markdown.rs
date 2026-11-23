#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownBlock {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph(String),
    CodeFence {
        language: Option<String>,
        code: String,
        closed: bool,
    },
    ListItem(String),
    Blockquote(String),
}

/// Incrementally splits streaming Markdown into stable blocks so completed paragraphs and
/// code fences do not need full-document re-parsing on every incoming delta.
pub fn parse_streaming_blocks(input: &str) -> Vec<MarkdownBlock> {
    let mut blocks = Vec::new();
    let mut paragraph_buf = Vec::new();
    let mut in_fence = false;
    let mut fence_lang: Option<String> = None;
    let mut fence_lines: Vec<String> = Vec::new();

    let flush_paragraph = |buf: &mut Vec<String>, out: &mut Vec<MarkdownBlock>| {
        if !buf.is_empty() {
            out.push(MarkdownBlock::Paragraph(buf.join("\n")));
            buf.clear();
        }
    };

    for line in input.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("```") {
            if in_fence {
                blocks.push(MarkdownBlock::CodeFence {
                    language: fence_lang.take(),
                    code: fence_lines.join("\n"),
                    closed: true,
                });
                fence_lines.clear();
                in_fence = false;
            } else {
                flush_paragraph(&mut paragraph_buf, &mut blocks);
                in_fence = true;
                let lang = rest.trim();
                fence_lang = if lang.is_empty() {
                    None
                } else {
                    Some(lang.to_string())
                };
            }
            continue;
        }

        if in_fence {
            fence_lines.push(line.to_string());
            continue;
        }

        if trimmed.is_empty() {
            flush_paragraph(&mut paragraph_buf, &mut blocks);
            continue;
        }

        if let Some(heading_text) = trimmed.strip_prefix("### ") {
            flush_paragraph(&mut paragraph_buf, &mut blocks);
            blocks.push(MarkdownBlock::Heading {
                level: 3,
                text: heading_text.to_string(),
            });
        } else if let Some(heading_text) = trimmed.strip_prefix("## ") {
            flush_paragraph(&mut paragraph_buf, &mut blocks);
            blocks.push(MarkdownBlock::Heading {
                level: 2,
                text: heading_text.to_string(),
            });
        } else if let Some(heading_text) = trimmed.strip_prefix("# ") {
            flush_paragraph(&mut paragraph_buf, &mut blocks);
            blocks.push(MarkdownBlock::Heading {
                level: 1,
                text: heading_text.to_string(),
            });
        } else if let Some(quote) = trimmed.strip_prefix("> ") {
            flush_paragraph(&mut paragraph_buf, &mut blocks);
            blocks.push(MarkdownBlock::Blockquote(quote.to_string()));
        } else if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            flush_paragraph(&mut paragraph_buf, &mut blocks);
            blocks.push(MarkdownBlock::ListItem(item.to_string()));
        } else {
            paragraph_buf.push(line.to_string());
        }
    }

    if in_fence {
        blocks.push(MarkdownBlock::CodeFence {
            language: fence_lang,
            code: fence_lines.join("\n"),
            closed: false,
        });
    } else {
        flush_paragraph(&mut paragraph_buf, &mut blocks);
    }

    blocks
}

#[derive(Debug, Default)]
pub struct StreamingMarkdownCache {
    committed_blocks: Vec<MarkdownBlock>,
    committed_byte_offset: usize,
}

impl StreamingMarkdownCache {
    /// Incrementally parses only newly appended content beyond the last committed boundary,
    /// avoiding O(N^2) re-parsing during high-frequency streaming deltas.
    pub fn update(&mut self, full_text: &str) -> Vec<MarkdownBlock> {
        if full_text.len() < self.committed_byte_offset {
            self.committed_blocks.clear();
            self.committed_byte_offset = 0;
        }
        let tail = &full_text[self.committed_byte_offset..];
        if let Some(split_pos) = tail.rfind("\n\n") {
            let safe_end = self.committed_byte_offset + split_pos + 2;
            let prefix = &full_text[self.committed_byte_offset..safe_end];
            if !prefix.contains("```") || prefix.matches("```").count().is_multiple_of(2) {
                self.committed_blocks.extend(parse_streaming_blocks(prefix));
                self.committed_byte_offset = safe_end;
            }
        }
        let mut out = self.committed_blocks.clone();
        out.extend(parse_streaming_blocks(
            &full_text[self.committed_byte_offset..],
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_headings_paragraphs_and_unclosed_streaming_fence() {
        let md = "# Title\n\nHello world\n\n```rust\nfn main() {}";
        let blocks = parse_streaming_blocks(md);
        assert_eq!(blocks.len(), 3);
        assert_eq!(
            blocks[0],
            MarkdownBlock::Heading {
                level: 1,
                text: "Title".into()
            }
        );
        assert_eq!(blocks[1], MarkdownBlock::Paragraph("Hello world".into()));
        assert_eq!(
            blocks[2],
            MarkdownBlock::CodeFence {
                language: Some("rust".into()),
                code: "fn main() {}".into(),
                closed: false
            }
        );
    }

    #[test]
    fn incremental_cache_matches_full_parse() {
        let full = "# Intro\n\nParagraph one.\n\n```rust\nlet x = 1;\n```\n\nTail line";
        let mut cache = StreamingMarkdownCache::default();
        let mut partial = String::new();
        for ch in full.chars() {
            partial.push(ch);
            let _ = cache.update(&partial);
        }
        assert_eq!(cache.update(full), parse_streaming_blocks(full));
    }
}
