//! Generic utilities

/// Transform a parameter value into an Option.
/// # Example
/// ```rust
/// use service_io::util::IntoOption;
///
/// fn foo(param: impl IntoOption<String>) -> Option<String> {
///    return param.into_some();
/// }
///
/// let expected = Some(String::from("data"));
///
/// assert_eq!(expected, foo(Some(String::from("data"))));
/// assert_eq!(expected, foo(String::from("data")));
/// assert_eq!(expected, foo(Some("data")));
/// assert_eq!(expected, foo("data"));
/// ```
pub trait IntoOption<T> {
    fn into_some(self) -> Option<T>;
}

impl<T> IntoOption<T> for T {
    fn into_some(self) -> Option<T> {
        Some(self)
    }
}

impl<T> IntoOption<T> for Option<T> {
    fn into_some(self) -> Option<T> {
        self
    }
}

impl<'a> IntoOption<String> for &'a str {
    fn into_some(self) -> Option<String> {
        Some(self.into())
    }
}

impl<'a> IntoOption<String> for Option<&'a str> {
    fn into_some(self) -> Option<String> {
        self.map(|s| s.into())
    }
}
