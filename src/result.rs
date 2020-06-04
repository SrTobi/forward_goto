use syn::spanned::Spanned;

pub type ErrInfo = (proc_macro2::Span, String);
pub type Result<T> = std::result::Result<T, ErrInfo>;

pub fn err(spanned: impl Spanned, msg: impl Into<String>) -> Result<()> {
    Err((spanned.span(), msg.into()))
}