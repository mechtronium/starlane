use alloc::string::{String, ToString};
use alloc::sync::Arc;
use core::ops::Deref;
use core::range::{Range, RangeFrom, RangeTo};
use nom::{AsBytes, AsChar, Compare, CompareResult, FindSubstring, IResult, InputIter, InputLength, InputTake, InputTakeAtPosition, Needed, Offset, Slice};
use nom::error::{ErrorKind, ParseError};
use crate::space::parse::util::{Input, Trace};



pub type LocatedSpan<'a> = nom_locate::LocatedSpan<&'a str,()>;




pub trait Input:
    Clone
    + ToString
    + AsBytes
    + Slice<Range<usize>>
    + Slice<RangeTo<usize>>
    + Slice<RangeFrom<usize>>
    + InputLength
    + Offset
    + InputTake
    + InputIter<Item = char>
    + InputTakeAtPosition<Item = char>
    + Compare<&'static str>
    + FindSubstring<&'static str>
    + core::fmt::Debug
where
    Self: Sized,
    <Self as InputTakeAtPosition>::Item: AsChar,
{
    fn location_offset(&self) -> usize;

    fn location_line(&self) -> u32;

    fn get_column(&self) -> usize;

    fn extra(&self) -> Arc<String>;

    fn len(&self) -> usize;

    fn range(&self) -> Range<usize>;

    fn trace(&self) -> Trace {
        Trace {
            range: self.range(),
            extra: self.extra(),
        }
    }
}

impl<'a> Input for Span<LocatedSpan<'a>> {
    fn location_offset(&self) -> usize {
        self.input.location_offset()
    }

    fn get_column(&self) -> usize {
        self.input.get_column()
    }

    fn location_line(&self) -> u32 {
        self.input.location_line()
    }

    fn extra(&self) -> () {
        ()
    }

    fn len(&self) -> usize {
        self.input.len()
    }

    fn range(&self) -> Range<usize> {
        Range {
            start: self.location_offset(),
            end: self.location_offset() + self.len(),
        }
    }
}




#[derive(Debug, Clone)]
pub struct Span<I>
where
    I: Clone
        + ToString
        + AsBytes
        + Slice<Range<usize>>
        + Slice<RangeTo<usize>>
        + Slice<RangeFrom<usize>>
        + InputLength
        + Offset
        + InputTake
        + InputIter<Item = char>
        + core::fmt::Debug
        + InputTakeAtPosition<Item = char>,
{
    input: I,
}

impl<I> Span<I>
where
    I: Clone
        + ToString
        + AsBytes
        + Slice<Range<usize>>
        + Slice<RangeTo<usize>>
        + Slice<RangeFrom<usize>>
        + InputLength
        + Offset
        + InputTake
        + InputIter<Item = char>
        + core::fmt::Debug
        + InputTakeAtPosition<Item = char>,
{
    pub fn new(input: I) -> Self {
        Self { input }
    }
}

impl<I> Deref for Span<I>
where
    I: Clone
        + ToString
        + AsBytes
        + Slice<Range<usize>>
        + Slice<RangeTo<usize>>
        + Slice<RangeFrom<usize>>
        + InputLength
        + Offset
        + InputTake
        + InputIter<Item = char>
        + core::fmt::Debug
        + InputTakeAtPosition<Item = char>,
{
    type Target = I;

    fn deref(&self) -> &Self::Target {
        &self.input
    }
}

impl<I> AsBytes for Span<I>
where
    I: Clone
        + ToString
        + AsBytes
        + Slice<Range<usize>>
        + Slice<RangeTo<usize>>
        + Slice<RangeFrom<usize>>
        + InputLength
        + Offset
        + InputTake
        + InputIter<Item = char>
        + core::fmt::Debug
        + InputTakeAtPosition<Item = char>,
{
    fn as_bytes(&self) -> &[u8] {
        self.input.as_bytes()
    }
}

impl<I> Slice<Range<usize>> for Span<I>
where
    I: Clone
        + ToString
        + AsBytes
        + Slice<Range<usize>>
        + Slice<RangeTo<usize>>
        + Slice<RangeFrom<usize>>
        + InputLength
        + Offset
        + InputTake
        + InputIter<Item = char>
        + core::fmt::Debug
        + InputTakeAtPosition<Item = char>,
{
    fn slice(&self, range: Range<usize>) -> Self {
        Self::new(self.input.slice(range))
    }
}

impl<I> Slice<RangeFrom<usize>> for Span<I>
where
    I: Clone
        + ToString
        + AsBytes
        + Slice<Range<usize>>
        + Slice<RangeTo<usize>>
        + Slice<RangeFrom<usize>>
        + InputLength
        + Offset
        + InputTake
        + InputIter<Item = char>
        + core::fmt::Debug
        + InputTakeAtPosition<Item = char>,
{
    fn slice(&self, range: RangeFrom<usize>) -> Self {
        Self::new(self.input.slice(range))
    }
}

impl<I> Slice<RangeTo<usize>> for Span<I>
where
    I: Clone
        + ToString
        + AsBytes
        + Slice<Range<usize>>
        + Slice<RangeTo<usize>>
        + Slice<RangeFrom<usize>>
        + InputLength
        + Offset
        + InputTake
        + InputIter<Item = char>
        + core::fmt::Debug
        + InputTakeAtPosition<Item = char>,
{
    fn slice(&self, range: RangeTo<usize>) -> Self {
        Self::new(self.input.slice(range))
    }
}

impl<'a> Compare<&'static str> for Span<LocatedSpan<'a>> {
    fn compare(&self, t: &str) -> CompareResult {
        self.input.compare(t)
    }

    fn compare_no_case(&self, t: &str) -> CompareResult {
        self.input.compare_no_case(t)
    }
}

impl<I> InputLength for Span<I>
where
    I: Clone
        + ToString
        + AsBytes
        + Slice<Range<usize>>
        + Slice<RangeTo<usize>>
        + Slice<RangeFrom<usize>>
        + InputLength
        + Offset
        + InputTake
        + InputIter<Item = char>
        + core::fmt::Debug
        + InputTakeAtPosition<Item = char>,
{
    fn input_len(&self) -> usize {
        self.input.input_len()
    }
}

impl<I> Offset for Span<I>
where
    I: Clone
        + ToString
        + AsBytes
        + Slice<Range<usize>>
        + Slice<RangeTo<usize>>
        + Slice<RangeFrom<usize>>
        + InputLength
        + Offset
        + InputTake
        + InputIter<Item = char>
        + core::fmt::Debug
        + InputTakeAtPosition<Item = char>,
{
    fn offset(&self, second: &Self) -> usize {
        self.input.offset(&second.input)
    }
}

impl<I> InputIter for Span<I>
where
    I: Clone
        + ToString
        + AsBytes
        + Slice<Range<usize>>
        + Slice<RangeTo<usize>>
        + Slice<RangeFrom<usize>>
        + InputLength
        + Offset
        + InputTake
        + InputIter<Item = char>
        + core::fmt::Debug
        + InputTakeAtPosition<Item = char>,
{
    type Item = <I as InputIter>::Item;
    type Iter = <I as InputIter>::Iter;
    type IterElem = <I as InputIter>::IterElem;

    fn iter_indices(&self) -> Self::Iter {
        self.input.iter_indices()
    }

    fn iter_elements(&self) -> Self::IterElem {
        self.input.iter_elements()
    }

    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Item) -> bool,
    {
        self.input.position(predicate)
    }

    fn slice_index(&self, count: usize) -> Result<usize, Needed> {
        self.input.slice_index(count)
    }
}

impl<I> InputTake for Span<I>
where
    I: Clone
        + ToString
        + AsBytes
        + Slice<Range<usize>>
        + Slice<RangeTo<usize>>
        + Slice<RangeFrom<usize>>
        + InputLength
        + Offset
        + InputTake
        + InputIter<Item = char>
        + core::fmt::Debug
        + InputTakeAtPosition<Item = char>,
{
    fn take(&self, count: usize) -> Self {
        Span::new(self.input.take(count))
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        let (left, right) = self.input.take_split(count);
        (Span::new(left), Span::new(right))
    }
}

impl<I> ToString for Span<I>
where
    I: Clone
        + ToString
        + AsBytes
        + Slice<Range<usize>>
        + Slice<RangeTo<usize>>
        + Slice<RangeFrom<usize>>
        + InputLength
        + Offset
        + InputTake
        + InputIter<Item = char>
        + core::fmt::Debug
        + InputTakeAtPosition<Item = char>,
{
    fn to_string(&self) -> String {
        self.input.to_string()
    }
}

impl<'a> FindSubstring<&str> for Span<LocatedSpan<'a>> {
    fn find_substring(&self, substr: &str) -> Option<usize> {
        self.input.find_substring(substr)
    }
}

impl<I> InputTakeAtPosition for Span<I>
where
    I: Clone
        + ToString
        + AsBytes
        + Slice<Range<usize>>
        + Slice<RangeTo<usize>>
        + Slice<RangeFrom<usize>>
        + InputLength
        + Offset
        + InputTake
        + InputIter<Item = char>
        + core::fmt::Debug
        + InputTakeAtPosition<Item = char>,
{
    type Item = <I as InputIter>::Item;

    fn split_at_position<P, E: ParseError<Self>>(&self, predicate: P) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.position(predicate) {
            Some(n) => Ok(self.take_split(n)),
            None => Err(nom::Err::Incomplete(Needed::new(1))),
        }
    }

    fn split_at_position1<P, E: ParseError<Self>>(
        &self,
        predicate: P,
        e: ErrorKind,
    ) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.position(predicate) {
            Some(0) => Err(nom::Err::Error(E::from_error_kind(self.clone(), e))),
            Some(n) => Ok(self.take_split(n)),
            None => Err(nom::Err::Incomplete(Needed::new(1))),
        }
    }

    fn split_at_position_complete<P, E: ParseError<Self>>(
        &self,
        predicate: P,
    ) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.split_at_position(predicate) {
            Err(nom::Err::Incomplete(_)) => Ok(self.take_split(self.input_len())),
            res => res,
        }
    }

    fn split_at_position1_complete<P, E: ParseError<Self>>(
        &self,
        predicate: P,
        e: ErrorKind,
    ) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.split_at_position1(predicate, e) {
            Err(nom::Err::Incomplete(_)) => {
                if self.input_len() == 0 {
                    Err(nom::Err::Error(E::from_error_kind(self.clone(), e)))
                } else {
                    Ok(self.take_split(self.input_len()))
                }
            }
            res => res,
        }
    }
}




pub enum Tag {
    SegSep,
    VarPrefix,
    CurlyOpen,
    CurlyClose,
    AngleOpen,
    AngleClose,
    SquareOpen,
    SquareClose,
    ParenOpen,
    ParenClose,
    DoubleQuote,
    SingleQuote,
    Slash,
    At,
    Bang,
    Question,
    Wildcard,
    BackTic,
    Pound,
    Plus,
    Minus
}

impl Tag {
    fn as_str(&self) -> &'static str {
        match self {
            Tag::SegSep => ":",
            Tag::VarPrefix => "$",
            Tag::CurlyOpen => "{",
            Tag::CurlyClose => "}",
            Tag::AngleOpen => "<",
            Tag::AngleClose => ">",
            Tag::SquareOpen => "[",
            Tag::SquareClose => "]",
            Tag::ParenOpen => "(",
            Tag::ParenClose=> ")",
            Tag::DoubleQuote => "\"",
            Tag::SingleQuote => "'",
            Tag::Slash => "/",
            Tag::At => "@",
            Tag::Bang => "!",
            Tag::Question => "?",
            Tag::Wildcard => "*",
            Tag::BackTic => "`",
            Tag::Pound => "#",
            Tag::Plus => "+",
            Tag::Minus => "-"
        }
    }
}
pub mod err {
    use alloc::format;
    use alloc::string::String;
    use core::range::Range;
    use nom_supreme::error::GenericErrorTree;
    use thiserror::Error;
    use crate::space::parse::ctx::ParseCtx;
    use crate::space::parse::nom::{Input, Tag};
    pub type ErrTree<'a,I: Input,Ctx:ParseCtx> = GenericErrorTree<I, Tag, Ctx, ParseErr<'a>>;

    pub struct ErrCtxStack {

    }

    #[derive(Error)]
    pub struct ParseErr<'a,M> where M: ErrMsg+'static {
        message: &'static dyn M,
        range: Range<usize>,
        span: &'a str
    }

    pub trait ErrMsg: 'static+Into<&str>
    {
        fn msg( &self, span: &str) -> &str;
    }


}