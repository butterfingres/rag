use {
    quick_xml::{
        events::Event as BytesEvent,
        name::QName,
        reader::{Reader as BytesReader, Span},
    },
    std::str,
};

pub struct Reader<'a>(BytesReader<&'a [u8]>);
impl<'a> Reader<'a> {
    pub fn from_str(s: &'a str) -> Self {
        Self(BytesReader::from_str(s))
    }

    pub fn read_event(&mut self) -> Result<Event<'a>, quick_xml::Error> {
        self.0.read_event().map(|ev| {
            // SAFETY: The [Reader] must be utf-8
            unsafe {
                match ev {
                    BytesEvent::Start(val) => Event::Start(Start::new_unchecked(val)),
                    BytesEvent::End(val) => Event::End(End::new_unchecked(val)),
                    BytesEvent::Empty(val) => Event::Empty(Start::new_unchecked(val)),
                    BytesEvent::Text(val) => Event::Text(Text::new_unchecked(val)),
                    BytesEvent::CData(val) => Event::CData(CData::new_unchecked(val)),
                    BytesEvent::Comment(val) => Event::Comment(Text::new_unchecked(val)),
                    BytesEvent::Decl(val) => Event::Decl(Decl::new_unchecked(val)),
                    BytesEvent::PI(val) => Event::PI(PI::new_unchecked(val)),
                    BytesEvent::DocType(val) => Event::DocType(Text::new_unchecked(val)),
                    BytesEvent::GeneralRef(val) => Event::GeneralRef(Ref::new_unchecked(val)),
                    BytesEvent::Eof => Event::Eof,
                }
            }
        })
    }

    pub fn read_to_end(&mut self, tag: &str) -> Result<Span, quick_xml::Error> {
        self.0.read_to_end(QName(tag.as_bytes()))
    }
}
impl<'a> Reader<'a> {
    pub fn as_str(&self) -> &'a str {
        unsafe { str::from_utf8_unchecked(&self.0.get_ref()) }
    }
}
impl<'a> std::ops::Deref for Reader<'a> {
    type Target = BytesReader<&'a [u8]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub enum Event<'a> {
    Start(Start<'a>),
    End(End<'a>),
    Empty(Start<'a>),
    Text(Text<'a>),
    CData(CData<'a>),
    Comment(Text<'a>),
    Decl(Decl<'a>),
    PI(PI<'a>),
    DocType(Text<'a>),
    GeneralRef(Ref<'a>),
    Eof,
}
macro_rules! def_wrapper {
    ($vis:vis struct $new:ident($old:ident)) => {
        $vis struct $new<'a>(::quick_xml::events::$old<'a>);
        impl<'a> $new<'a> {
            /// # Safety
            ///
            /// `ev` must be utf-8
            pub unsafe fn new_unchecked(ev: ::quick_xml::events::$old<'a>) -> Self {
                // SAFETY: `ev` must be utf-8
                Self(ev)
            }
        }
        impl<'a> ::core::ops::Deref for $new<'a> {
            type Target = ::quick_xml::events::$old<'a>;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        impl<'a> ::core::convert::From<$new<'a>> for ::quick_xml::events::$old<'a> {
            fn from($new(val): $new<'a>) -> Self {
                val
            }
        }
    }
}
def_wrapper!(pub struct Start(BytesStart));
def_wrapper!(pub struct End(BytesEnd));
def_wrapper!(pub struct Text(BytesText));
def_wrapper!(pub struct CData(BytesCData));
def_wrapper!(pub struct Decl(BytesDecl));
def_wrapper!(pub struct PI(BytesPI));
def_wrapper!(pub struct Ref(BytesRef));
impl<'a> Start<'a> {
    pub fn name(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.0.name().0) }
    }
    pub fn local_name(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.0.local_name().into_inner()) }
    }
}
impl<'a> End<'a> {
    pub fn name(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.0.name().0) }
    }
    pub fn local_name(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.0.local_name().into_inner()) }
    }
}
impl Ref<'_> {
    pub fn as_ref_name(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.0.as_ref()) }
    }
}
impl AsRef<str> for CData<'_> {
    fn as_ref(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.0.as_ref()) }
    }
}
impl AsRef<str> for Text<'_> {
    fn as_ref(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.0.as_ref()) }
    }
}
