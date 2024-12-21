//! A markdown parser processor
//!
//! Utilizing pulldown-cmark.

use std::sync::LazyLock;

use htmd::HtmlToMarkdown;
use pulldown_cmark::Options;

/// The following markdown options are enabled based on what Discord supports.
static MARKDOWN_OPTIONS: LazyLock<Options> = LazyLock::new(|| {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);
    opts
});

pub(crate) fn expand_url(url: &str, base_url: &url::Url) -> Result<String, url::ParseError> {
    // expand relative URLs if possible
    let joined = base_url.join(url)?;
    Ok(joined.to_string())
}

/// Convert HTML to markdown
pub fn html_to_markdown(html: &str, base_url: &url::Url) -> Result<String, std::io::Error> {
    let base_url = base_url.clone();
    let converter = HtmlToMarkdown::builder()
        .options(htmd::options::Options {
            hr_style: htmd::options::HrStyle::Dashes,
            bullet_list_marker: htmd::options::BulletListMarker::Dash,
            heading_style: htmd::options::HeadingStyle::Atx,
            ..Default::default()
        })
        .skip_tags(vec!["script", "style"])
        .add_handler(vec!["a", "img"], move |el: htmd::Element| {
            let mut attrs = el.attrs.iter();
            let src = attrs.find(|&attr| format!("{:?}", attr.name.expanded()) == "src");
            let alt = attrs.find(|&attr| format!("{:?}", attr.name.expanded()) == "alt");
            let href = attrs.find(|&attr| format!("{:?}", attr.name.expanded()) == "href");
            let title = attrs.find(|&attr| format!("{:?}", attr.name.expanded()) == "title");

            match el.tag {
                "img" => match (src, alt) {
                    (Some(src), Some(alt)) => {
                        let src = src.value.to_string();
                        let alt = alt.value.to_string();

                        let expand_src = expand_url(&src, &base_url).unwrap_or(src);

                        if let Some(title) = title {
                            let title = title.value.to_string();
                            Some(format!("![{alt}]({expand_src} \"{title}\")"))
                        } else {
                            Some(format!("![{alt}]({expand_src})"))
                        }
                    }
                    (Some(src), None) => {
                        let src = src.value.to_string();

                        let expand_src = expand_url(&src, &base_url).unwrap_or(src);

                        if let Some(title) = title {
                            let title = title.value.to_string();
                            Some(format!("![]({expand_src} \"{title}\")"))
                        } else {
                            Some(format!("![]({expand_src})"))
                        }
                    }
                    _ => None,
                },
                "a" => match (href, title) {
                    (Some(href), Some(title)) => {
                        let href = href.value.to_string();
                        let title = title.value.to_string();

                        let expand_href = expand_url(&href, &base_url).unwrap_or(href);

                        Some(format!("[{title}]({expand_href})"))
                    }
                    (Some(href), None) => {
                        let href = href.value.to_string();
                        let content = el.content;

                        let expand_href = expand_url(&href, &base_url).unwrap_or(href);

                        Some(format!("[{content}]({expand_href})"))
                    }
                    _ => None,
                },
                _ => None,
            }
        })
        .add_handler(vec!["svg"], |_: htmd::Element| {
            Some("[SVG Image]".to_string())
        })
        .build();

    converter.convert(html)
}

/// Convert markdown to HTML
pub fn markdown_to_html(markdown: &str) -> String {
    let mut html = String::new();
    let parser = pulldown_cmark::Parser::new_ext(markdown, *MARKDOWN_OPTIONS);
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

/// Convert markdown to plain text
pub fn markdown_to_text(markdown: &str) -> String {
    let mut text = String::new();
    let parser = pulldown_cmark::Parser::new_ext(markdown, *MARKDOWN_OPTIONS);
    self::plain_text::push_plain_text(&mut text, parser);
    text
}

mod plain_text {
    use pulldown_cmark_escape::FmtWriter;

    struct PlainTextWriter<'a, I, W> {
        iter: I,
        writer: W,
        end_newline: bool,

        _marker: std::marker::PhantomData<&'a ()>,
    }

    impl<'a, I, W> PlainTextWriter<'a, I, W>
    where
        I: Iterator<Item = pulldown_cmark::Event<'a>>,
        W: pulldown_cmark_escape::StrWrite,
    {
        fn new(iter: I, writer: W) -> Self {
            PlainTextWriter {
                iter,
                writer,
                end_newline: true,
                _marker: std::marker::PhantomData,
            }
        }

        #[inline]
        fn write_newline(&mut self) -> Result<(), W::Error> {
            self.end_newline = true;
            self.writer.write_str("\n")
        }

        #[inline]
        fn write(&mut self, s: &str) -> Result<(), W::Error> {
            self.writer.write_str(s)?;

            if !s.is_empty() {
                self.end_newline = s.ends_with('\n');
            }

            Ok(())
        }

        pub fn run(mut self) -> Result<(), W::Error> {
            while let Some(event) = self.iter.next() {
                match event {
                    pulldown_cmark::Event::Start(_) => {}
                    pulldown_cmark::Event::End(_) => {}
                    pulldown_cmark::Event::Text(text) => self.write(&text)?,
                    pulldown_cmark::Event::Code(text) => self.write(&text)?,
                    pulldown_cmark::Event::Html(text) => self.write(&text)?,
                    pulldown_cmark::Event::InlineHtml(text) => self.write(&text)?,
                    pulldown_cmark::Event::DisplayMath(_) => {}
                    pulldown_cmark::Event::InlineMath(_) => {}
                    pulldown_cmark::Event::FootnoteReference(_) => {}
                    pulldown_cmark::Event::SoftBreak => self.write_newline()?,
                    pulldown_cmark::Event::HardBreak => self.write_newline()?,
                    pulldown_cmark::Event::Rule => self.write_newline()?,
                    pulldown_cmark::Event::TaskListMarker(_) => {}
                }
            }

            Ok(())
        }
    }

    pub fn push_plain_text<'a, I>(s: &mut String, iter: I)
    where
        I: Iterator<Item = pulldown_cmark::Event<'a>>,
    {
        if let Ok(()) = write_plain_text_fmt(s, iter) {}
    }

    fn write_plain_text_fmt<'a, I, W>(writer: W, iter: I) -> std::fmt::Result
    where
        I: Iterator<Item = pulldown_cmark::Event<'a>>,
        W: std::fmt::Write,
    {
        PlainTextWriter::new(iter, FmtWriter(writer)).run()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_to_html() {
        let markdown = r#"Hello **world**!"#;

        let html = markdown_to_html(markdown);
        assert_eq!(html, "<p>Hello <strong>world</strong>!</p>\n");
    }

    #[test]
    fn test_markdown_to_plain_text() {
        let markdown = r#"Hello **world**!"#;

        let text = markdown_to_text(markdown);
        assert_eq!(text, "Hello world!");
    }
}
