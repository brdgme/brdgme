use crate::ast::TNode;

pub fn render(input: &[TNode]) -> String {
    let mut buf = String::new();
    for n in input {
        match *n {
            TNode::Text(ref t) => buf.push_str(t),
            TNode::Fg(_, ref children) | TNode::Bg(_, ref children) | TNode::Bold(ref children) => {
                buf.push_str(&render(children));
            }
        }
    }
    buf
}
