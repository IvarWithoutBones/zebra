use core::{iter, str};

pub const SEPARATOR: char = '/';

#[derive(Debug, PartialEq, Eq)]
pub enum PathComponent<'a> {
    Separator,
    CurrentDir,
    ParentDir,
    Normal(&'a str),
}

impl<'a> PathComponent<'a> {
    pub fn parse(buf: &'a [u8]) -> Option<(usize, Self)> {
        match buf {
            [b'.', b'.', ..] => Some((2, Self::ParentDir)),
            [b'/', ..] => Some((1, Self::Separator)),
            [b'.', ..] => Some((1, Self::CurrentDir)),

            _ if !buf.is_empty() => buf
                .iter()
                .position(|&c| c == SEPARATOR as u8)
                .map(|end| (end, Self::Normal(str::from_utf8(&buf[..end]).unwrap())))
                .or_else(|| Some((buf.len(), Self::Normal(str::from_utf8(buf).unwrap())))),

            _ => None,
        }
    }

    pub const fn as_str(&self) -> &str {
        match self {
            Self::Separator => "/",
            Self::CurrentDir => ".",
            Self::ParentDir => "..",
            Self::Normal(s) => s,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct Path<'a> {
    inner: &'a str,
}

impl<'a> Path<'a> {
    pub fn new<Str>(s: &'a Str) -> Self
    where
        Str: AsRef<str> + ?Sized,
    {
        Self { inner: s.as_ref() }
    }

    pub fn as_str(&self) -> &str {
        self.inner
    }

    pub fn components(&self) -> impl Iterator<Item = PathComponent> + '_ {
        let buf = self.inner.as_bytes();
        let mut start = 0;

        iter::from_fn(move || {
            let (end, component) = PathComponent::parse(&buf[start..])?;
            start += end;
            Some(component)
        })
    }

    pub fn is_absolute(&self) -> bool {
        self.components().next() == Some(PathComponent::Separator)
    }

    pub fn is_relative(&self) -> bool {
        !self.is_absolute()
    }

    pub fn is_directory(&self) -> bool {
        match self.components().last() {
            Some(PathComponent::Separator) => true,
            Some(PathComponent::CurrentDir) => true,
            Some(PathComponent::ParentDir) => true,
            Some(PathComponent::Normal(s)) => s.ends_with(SEPARATOR),
            _ => false,
        }
    }

    pub fn is_file(&self) -> bool {
        !self.is_directory()
    }

    pub fn depth(&self) -> usize {
        self.components()
            .filter(|c| c == &PathComponent::Separator)
            .count()
    }
}

impl<'a> AsRef<str> for Path<'a> {
    fn as_ref(&self) -> &str {
        self.inner
    }
}

impl<'a> From<&'a str> for Path<'a> {
    fn from(s: &'a str) -> Self {
        Self { inner: s }
    }
}
