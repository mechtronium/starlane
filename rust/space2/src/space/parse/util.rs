use crate::lib::std::sync::Arc;
use crate::lib::std::string::{String, ToString};
use crate::lib::std::fmt::{Display};

use crate::lib::std::ops::{Deref, RangeTo,RangeFrom,Range};

use crate::space::parse::case::VarCase;
use crate::space::parse::ctx::{InputCtx, PrimCtx};
use crate::space::parse::nomplus::{ErrTree, Input, LocatedSpan, Res, Span};
use core::error::Error as RustErr;
use nom::error::{ErrorKind, ParseError};
use nom::{AsBytes, Compare, CompareResult, FindSubstring, IResult, InputIter, InputLength, InputTake, InputTakeAtPosition, Offset, Slice};
use nom::character::complete::multispace0;
use nom::sequence::delimited;
use nom_supreme::error::StackContext;
use nom_supreme::ParserExt;
use crate::space::parse::err::{ParseErrs, ParseErrsDef};
use crate::space::parse::nomplus::err::ParseErr;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

// TraceWrap
pub struct Trace<W> {
    pub range: Range<usize>,
    pub w: W,
}


impl<W> Trace<W> {
    pub fn new<I>(input :I, w: W) -> Self where I: Input{
        Self {
            range: input.range(),
            w,
        }
    }

    pub fn from_range<N>( range: Range<usize>, w: N ) -> Trace<N> {
        Trace {
            range,
            w
        }
    }

    pub fn unwrap(self) -> W {
        self.w
    }
}

impl<W> PartialEq<W> for Trace<W>
where
    W: PartialEq<W>,
{
    fn eq(&self, other: &W) -> bool {
       self.w == other
    }
}

impl<W> PartialEq<Self> for Trace<W>
where
    W: PartialEq<W>,
{
    fn eq(&self, other: &Self) -> bool {
        self.range == other.range && self.w == other.w
    }
}

impl <W> Eq for Trace<W> where W: Eq {

}

impl <W> Clone for Trace<W> where W: Clone {
    fn clone(&self) -> Self {
        Self {
            range: self.range.clone(),
            w: self.w.clone(),
        }
    }
}

impl<W> ToString for Trace<W>
where
    W: ToString,
{
    fn to_string(&self) -> String {
        self.w.to_string()
    }
}

impl<W> Deref for Trace<W> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.w
    }
}

/*
impl Into<Variable> for Trace<VarCase> {
    fn into(self) -> Variable {
        Trace::from_range(self.range,VarCase(self.w.to_string()))
    }
}

 */

pub fn tron<I, F, O>(mut f: F) -> impl FnMut(I) -> Res<I,Trace<O>>
where
    I: Input,
    F: FnMut(I) -> Res<I, O>,
{
    move |input: I| {
        let (next, output) = f(input.clone())?;

        let range = Range::from(0..next.len());
        let span = input.slice(range);
        let tw = Trace::new(span, output);

        Ok((next, tw))
    }
}

//pub type OwnedSpan<'a> = LocatedSpan<&'a str, SpanExtra>;
pub type SpanExtra = Arc<String>;

pub fn new_span<'a>(s: &'a str) -> Span<LocatedSpan<'a>> {
    let span = LocatedSpan::new(s);
    Span::new(span)
}






#[derive(Debug, Clone)]
pub struct SliceStr {
    location_offset: usize,
    len: usize,
    string: Arc<String>,
}

impl ToString for SliceStr {
    fn to_string(&self) -> String {
        self.string
            .as_str()
            .slice(self.location_offset..self.location_offset + self.len)
            .to_string()
    }
}

impl SliceStr {
    pub fn new(string: String) -> Self {
        Self::from_arc(Arc::new(string))
    }

    pub fn from_arc(string: Arc<String>) -> Self {
        Self {
            len: string.len(),
            string,
            location_offset: 0,
        }
    }

    pub fn from(string: Arc<String>, location_offset: usize, len: usize) -> Self {
        Self {
            string,
            location_offset,
            len,
        }
    }
}

impl SliceStr {
    pub fn as_str(&self) -> &str {
        &self
            .string
            .as_str()
            .slice(self.location_offset..self.location_offset + self.len)
    }
}

impl Deref for SliceStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsBytes for SliceStr {
    fn as_bytes(&self) -> &[u8] {
        self.string
            .as_bytes()
            .slice(self.location_offset..self.location_offset + self.len)
    }
}

impl Slice<Range<usize>> for SliceStr {
    fn slice(&self, range: Range<usize>) -> Self {
        SliceStr {
            location_offset: self.location_offset + range.start,
            len: range.end - range.start,
            string: self.string.clone(),
        }
    }
}

impl Slice<RangeFrom<usize>> for SliceStr {
    fn slice(&self, range: RangeFrom<usize>) -> Self {
        SliceStr {
            location_offset: self.location_offset + range.start,
            len: self.len - range.start,
            string: self.string.clone(),
        }
    }
}

impl Slice<RangeTo<usize>> for SliceStr {
    fn slice(&self, range: RangeTo<usize>) -> Self {
        SliceStr {
            location_offset: self.location_offset,
            len: range.end,
            string: self.string.clone(),
        }
    }
}

impl Compare<&str> for SliceStr {
    fn compare(&self, t: &str) -> CompareResult {
        self.as_str().compare(t)
    }

    fn compare_no_case(&self, t: &str) -> CompareResult {
        self.as_str().compare_no_case(t)
    }
}

impl InputLength for SliceStr {
    fn input_len(&self) -> usize {
        self.len
    }
}

impl Offset for SliceStr {
    fn offset(&self, second: &Self) -> usize {
        self.location_offset
    }
}

pub struct MyCharIterator {}

pub struct MyChars {
    index: usize,
    slice: SliceStr,
}

impl MyChars {
    pub fn new(slice: SliceStr) -> Self {
        Self { index: 0, slice }
    }
}

impl Iterator for MyChars {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let mut chars = self.slice.as_str().chars();
        let next = chars.nth(self.index);
        match next {
            None => None,
            Some(next) => {
                self.index = self.index + 1;
                Some(next)
            }
        }
    }
}

pub struct CharIterator {
    index: usize,
    slice: SliceStr,
}

impl CharIterator {
    pub fn new(slice: SliceStr) -> Self {
        Self { index: 0, slice }
    }
}

impl Iterator for CharIterator {
    type Item = (usize, char);

    fn next(&mut self) -> Option<Self::Item> {
        let mut chars = self.slice.as_str().chars();
        let next = chars.nth(self.index);
        match next {
            None => None,
            Some(next) => {
                //let byte_index = self.index * std::mem::size_of::<char>();
                let byte_index = self.index;
                self.index = self.index + 1;
                Some((byte_index, next))
            }
        }
    }
}

impl InputIter for SliceStr {
    type Item = char;
    type Iter = CharIterator;
    type IterElem = MyChars;

    #[inline]
    fn iter_indices(&self) -> Self::Iter {
        CharIterator::new(self.clone())
    }
    #[inline]
    fn iter_elements(&self) -> Self::IterElem {
        MyChars::new(self.clone())
    }
    #[inline]
    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Item) -> bool,
    {
        self.as_str().position(predicate)
    }

    #[inline]
    fn slice_index(&self, count: usize) -> Result<usize, nom::Needed> {
        self.as_str().slice_index(count)
    }
}

impl InputTakeAtPosition for SliceStr {
    type Item = char;

    fn split_at_position<P, E: ParseError<Self>>(&self, predicate: P) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.split_at_position(predicate) {
            Err(nom::Err::Incomplete(_)) => Ok(self.take_split(self.input_len())),
            res => res,
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
        match self.as_str().position(predicate) {
            Some(0) => Err(nom::Err::Error(E::from_error_kind(self.clone(), e))),
            Some(n) => Ok(self.take_split(n)),
            None => Err(nom::Err::Incomplete(nom::Needed::new(1))),
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
        match self.as_str().position(predicate) {
            Some(0) => Err(nom::Err::Error(E::from_error_kind(self.clone(), e))),
            Some(n) => Ok(self.take_split(n)),
            None => {
                if self.as_str().input_len() == 0 {
                    Err(nom::Err::Error(E::from_error_kind(self.clone(), e)))
                } else {
                    Ok(self.take_split(self.input_len()))
                }
            }
        }
    }
}

impl InputTake for SliceStr {
    fn take(&self, count: usize) -> Self {
        self.slice(count..)
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        (self.slice(count..), self.slice(..count))
    }
}

impl FindSubstring<&str> for SliceStr {
    fn find_substring(&self, substr: &str) -> Option<usize> {
        self.as_str().find_substring(substr)
    }
}

#[cfg(test)]
pub mod test {
    use crate::lib::std::string::ToString;
    use nom::Slice;
    use crate::space::parse::util::SliceStr;

    #[test]
    pub fn test() {
        let s = SliceStr::new("abc123".to_string());
        assert_eq!(6, s.len());

        let s = s.slice(0..3);
        assert_eq!(3, s.len());
        assert_eq!("abc", s.as_str());


        let s = SliceStr::new("abc123".to_string());
        assert_eq!("123", s.slice(3..).as_str());
        assert_eq!("abc", s.slice(..3).as_str());
    }
}


pub fn wrap<I, F, O>(mut f: F) -> impl FnMut(I) -> Res<I, O>
where
    I: Input,
    F: FnMut(I) -> Res<I, O> + Copy

{
    move |input: I| f(input)
}

pub fn len<I, F, O>(f: F) -> impl FnMut(I) -> usize
where
    I: Input,
    F: FnMut(I) -> Res< I, O> + Copy,
{
    move |input: I| match recognize(wrap(f))(input) {
        Ok((_, span)) => span.len(),
        Err(_) => 0,
    }
}

pub fn trim< I, F, O, C, E>(f: F) -> impl FnMut(I) -> Res<I, O>
where
    I: Input,
    F: FnMut(I) -> Res< I, O> + Copy,
{
    move |input: I| delimited(multispace0, f, multispace0)(input)
}

pub fn result<I: Input, R>(result: Result<(I, R), nom::Err<ErrTree<I>>>) -> Result<R, ParseErr> {
    todo!()
    /*
    match result {
        Ok((_, e)) => Ok(e),
        Err(nom::Err::Error(err)) => {
            Result::Err(err.into())
        }
        Err(nom::Err::Failure(err)) => {
            Result::Err(err.into())
        }
        _ =>  {
            Result::Err(ParseErrs::new(&"Unidentified nom parse error"))
        }

    }

     */
}


pub fn parse_errs<'a,R,E>(result: Result<R,E>) -> Result<R, ParseErrs<'a>> where E: Display {
    match result {
        Ok(ok) => Ok(ok),
        Err(err) => Err(todo!())
    }
}

pub fn unstack( ctx: &StackContext<InputCtx>) -> String {
    match ctx {
        StackContext::Kind(k) => {
            k.description().to_string()
        }
        StackContext::Context(c) => {
            c.to_string()
        }
    }
}


pub fn recognize<I: Clone + Offset + Slice<RangeTo<usize>>, O, E: ParseError<I>, F>(
    mut parser: F,
) -> impl FnMut(I) -> IResult<I, I, E>
where
    F: ParserExt<I, O, E>,
{
    move |input: I| {
        let i = input.clone();
        match parser.parse(i) {
            Ok((i, _)) => {
                let index = input.offset(&i);
                Ok((i, input.slice(..index)))
            }
            Err(e) => Err(e),
        }
    }
}


pub fn log_parse_err<I,O>( result: Res<I,O>) -> Res<I,O> where I: Input
{

    if let Result::Err(err) = &result {
        match err {
            nom::Err::Incomplete(_) => {}
            nom::Err::Error(e) => print(e),
            nom::Err::Failure(e) => print(e)
        }
    }
    result
}

pub fn print<I>(err: &ErrTree<I>) where I: Input
{
    todo!()
    /*

    match err {
        ErrTree::Base { .. } => {
            println!("BASE!");
        }
        ErrTree::Stack { base,contexts } => {

            println!("STACK!");
            let mut contexts = contexts.clone();
            contexts.reverse();
            let mut message = String::new();

            if !contexts.is_empty()  {
                if let (location,err) = contexts.remove(0) {
                    let mut last = &err;
                    println!("line {} column: {}",location.location_line(), location.get_column());
                    let line = unstack(&err);
                    message.push_str(line.as_str());

                    for (span,context) in contexts.iter() {
                        last = context;
                        let line = format!("\n\t\tcaused by: {}",unstack(&context));
                        message.push_str(line.as_str());
                    }
                    ParseErrs::from_loc_span(message.as_str(), last.to_string(), location ).print();
                }
            }
        }
        ErrTree::Alt(_) => {
            println!("ALT!");
        }
    }

     */

}

pub fn preceded<I, O1, O2, E: ParseError<I>, F, G>(
    mut first: F,
    mut second: G,
) -> impl FnMut(I) -> IResult<I, O2, E>
where
    F: ParserExt<I, O1, E>,
    G: ParserExt<I, O2, E>,
{
    move |input: I| {
        let (input, _) = first.parse(input)?;
        second.parse(input)
    }
}



