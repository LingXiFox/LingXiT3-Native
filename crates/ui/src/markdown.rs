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
}
