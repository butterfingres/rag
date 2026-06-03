use {
    memchr::{Memchr, memchr_iter},
    std::iter,
};

pub trait Split {
    type Iter<'a>: Iterator<Item = &'a str>;

    fn split<'a>(_: &'a str) -> Self::Iter<'a>;
}

pub struct AsciiSplitter<const CHAR: u8>;
pub struct AsciiSplitterIter<'a, const CHAR: u8> {
    text: &'a str,
    iter: iter::Chain<Memchr<'a>, iter::Once<usize>>,
    cursor: usize,
}
impl<'a, const CHAR: u8> Iterator for AsciiSplitterIter<'a, CHAR> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|next| {
            let text = &self.text[self.cursor..next];
            self.cursor = next + 1;
            text
        })
    }
}
impl Split for AsciiSplitter<b' '> {
    type Iter<'a> = AsciiSplitterIter<'a, b' '>;

    fn split<'a>(text: &'a str) -> Self::Iter<'a> {
        AsciiSplitterIter {
            text,
            iter: memchr_iter(b' ', text.as_bytes()).chain(iter::once(text.len())),
            cursor: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_splitter_iter() {
        let mut iter = AsciiSplitter::<b' '>::split("hello world");
        assert_eq!(iter.next(), Some("hello"));
        assert_eq!(iter.next(), Some("world"));
        assert_eq!(iter.next(), None);

        let mut iter = AsciiSplitter::<b' '>::split("hello");
        assert_eq!(iter.next(), Some("hello"));
        assert_eq!(iter.next(), None);

        let mut iter = AsciiSplitter::<b' '>::split("");
        assert_eq!(iter.next(), Some(""));
        assert_eq!(iter.next(), None);
    }
}
