use brdgme_color::Color;

use crate::ast::TNode;

fn fg(color: Color, content: &str) -> String {
    format!(r#"<span style="color:{};">{}</span>"#, color, content)
}

fn bg(color: Color, content: &str) -> String {
    format!(
        r#"<span style="background-color:{};">{}</span>"#,
        color, content
    )
}

fn b(content: &str) -> String {
    format!("<b>{}</b>", content)
}

fn escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn render(input: &[TNode]) -> String {
    render_nodes(input)
}

fn render_nodes(input: &[TNode]) -> String {
    let mut buf = String::new();
    for n in input {
        match *n {
            TNode::Text(ref t) => buf.push_str(&escape(t)),
            TNode::Fg(color, ref children) => buf.push_str(&fg(color, &render_nodes(children))),
            TNode::Bg(color, ref children) => buf.push_str(&bg(color, &render_nodes(children))),
            TNode::Bold(ref children) => buf.push_str(&b(&render_nodes(children))),
        }
    }
    buf
}
