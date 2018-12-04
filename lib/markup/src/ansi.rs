use crate::ast::TNode;
use brdgme_color::Style;

pub fn render(input: &[TNode]) -> String {
    let default_style = Style::default();
    format!(
        "{}{}",
        default_style.ansi(),
        render_styled(input, default_style)
    )
}

fn render_styled(input: &[TNode], last_style: Style) -> String {
    let mut buf = String::new();
    for n in input {
        match *n {
            TNode::Text(ref t) => buf.push_str(t),
            TNode::Fg(ref color, ref children) => {
                let new_style = Style {
                    fg: color,
                    ..last_style
                };
                buf.push_str(&new_style.ansi());
                buf.push_str(&render_styled(children, new_style));
                buf.push_str(&last_style.ansi());
            }
            TNode::Bg(ref color, ref children) => {
                let new_style = Style {
                    bg: color,
                    ..last_style
                };
                buf.push_str(&new_style.ansi());
                buf.push_str(&render_styled(children, new_style));
                buf.push_str(&last_style.ansi());
            }
            TNode::Bold(ref children) => {
                let new_style = Style {
                    bold: true,
                    ..last_style
                };
                buf.push_str(&new_style.ansi());
                buf.push_str(&render_styled(children, new_style));
                buf.push_str(&last_style.ansi());
            }
        }
    }
    buf
}
