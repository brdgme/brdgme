use failure::Fail;

#[derive(Debug, Fail)]
pub enum MarkupError {
    #[fail(display = "failed to parse input")]
    Parse,
}