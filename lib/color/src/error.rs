use failure::Fail;

#[derive(Debug, Fail)]
pub enum ColorError {
    #[fail(display = "parse error: {}", message)] Parse { message: String },
}
