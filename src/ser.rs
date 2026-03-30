use crate::libyml::emitter::{
    Emitter, Event, Mapping, Scalar, ScalarStyle, Sequence,
};
use crate::modules::error::{Error, ErrorImpl, Result};
use serde::ser::{self};
use std::fmt::Display;
use std::io;

/// A structure for serializing Rust values into YAML.
///
/// # Example
///
/// ```
/// use serde::Serialize;
/// use std::collections::BTreeMap;
///
/// fn main() -> serde_yml::Result<()> {
///     let mut buffer = Vec::new();
///     let mut ser = serde_yml::Serializer::new(&mut buffer);
///
///     let mut object = BTreeMap::new();
///     object.insert("k", 107);
///     object.serialize(&mut ser)?;
///
///     object.insert("J", 74);
///     object.serialize(&mut ser)?;
///
///     assert_eq!(buffer, b"k: 107\n---\nJ: 74\nk: 107\n");
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct Serializer<'a, W> {
    /// The configuration of the serializer.
    pub config: SerializerConfig,
    /// The depth of the current serialization.
    pub depth: usize,
    /// The current state of the serializer.
    pub state: State,
    /// The YAML emitter.
    pub emitter: Emitter<'a, W>,
}

/// The configuration of the serializer.
#[derive(Copy, Clone, Debug, Default)]
pub struct SerializerConfig {
    /// When set to `true`, all unit variants will be serialized as tags, i.e. `!Unit` instead of `Unit`.
    pub tag_unit_variants: bool,
}

/// The state of the serializer.
#[derive(Debug)]
pub enum State {
    /// Nothing in particular.
    NothingInParticular,
    /// Check for a tag.
    CheckForTag,
    /// Check for a duplicate tag.
    CheckForDuplicateTag,
    /// Found a tag.
    FoundTag(String),
    /// Already tagged.
    AlreadyTagged,
}

impl<'a, W> Serializer<'a, W>
where
    W: io::Write + 'a,
{
    /// Creates a new YAML serializer.
    pub fn new(writer: W) -> Self {
        Self::new_with_config(writer, SerializerConfig::default())
    }

    /// Creates a new YAML serializer with a configuration.
    pub fn new_with_config(
        writer: W,
        config: SerializerConfig,
    ) -> Self {
        let mut emitter = Emitter::new(writer);
        emitter.emit(Event::StreamStart).unwrap();
        Serializer {
            config,
            depth: 0,
            state: State::NothingInParticular,
            emitter,
        }
    }

    /// Calls [`.flush()`](io::Write::flush) on the underlying `io::Write`
    /// object.
    pub fn flush(&mut self) -> Result<()> {
        self.emitter.flush().map_err(Error::from)?;
        Ok(())
    }

    /// Unwrap the underlying `io::Write` object from the `Serializer`.
    pub fn into_inner(mut self) -> Result<W> {
        self.emitter.emit(Event::StreamEnd).map_err(Error::from)?;
        self.emitter.flush().map_err(Error::from)?;
        Ok(self.emitter.into_inner())
    }

    /// Emit a scalar value.
    pub fn emit_scalar(
        &mut self,
        mut scalar: Scalar<'_>,
    ) -> Result<()> {
        self.flush_mapping_start()?;
        if let Some(tag) = self.take_tag() {
            scalar.tag = Some(tag);
        }
        self.value_start()?;
        self.emitter.emit(Event::Scalar(scalar)).map_err(Error::from)?;
        self.value_end()
    }

    /// Emit a sequence start.
    pub fn emit_sequence_start(&mut self) -> Result<()> {
        self.flush_mapping_start()?;
        self.value_start()?;
        let tag = self.take_tag();
        self.emitter
            .emit(Event::SequenceStart(Sequence { tag }))
            .map_err(Error::from)?;
        self.depth += 1;
        Ok(())
    }

    /// Emit a sequence end.
    pub fn emit_sequence_end(&mut self) -> Result<()> {
        self.emitter.emit(Event::SequenceEnd).map_err(Error::from)?;
        self.depth -= 1;
        self.value_end()
    }

    /// Emit a mapping start.
    pub fn emit_mapping_start(&mut self) -> Result<()> {
        self.flush_mapping_start()?;
        self.value_start()?;
        let tag = self.take_tag();
        self.emitter
            .emit(Event::MappingStart(Mapping { tag }))
            .map_err(Error::from)?;
        self.depth += 1;
        Ok(())
    }

    /// Emit a mapping end.
    pub fn emit_mapping_end(&mut self) -> Result<()> {
        self.emitter.emit(Event::MappingEnd).map_err(Error::from)?;
        self.depth -= 1;
        self.value_end()
    }

    /// Start of a value.
    pub fn value_start(&mut self) -> Result<()> {
        Ok(())
    }

    /// End of a value.
    pub fn value_end(&mut self) -> Result<()> {
        Ok(())
    }

    fn flush_mapping_start(&mut self) -> Result<()> {
        if let State::CheckForTag = self.state {
            self.state = State::NothingInParticular;
            self.emit_mapping_start()?;
        } else if let State::CheckForDuplicateTag = self.state {
            self.state = State::NothingInParticular;
        }
        Ok(())
    }

    /// Takes the tag from the serializer state.
    pub fn take_tag(&mut self) -> Option<String> {
        if let State::FoundTag(tag) =
            std::mem::replace(&mut self.state, State::NothingInParticular)
        {
            Some(tag)
        } else {
            None
        }
    }
}

impl<'a, W> ser::Serializer for &mut Serializer<'a, W>
where
    W: io::Write + 'a,
{
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.emit_scalar(Scalar {
            tag: None,
            value: if v { "true" } else { "false" },
            style: ScalarStyle::Plain,
        })
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        let mut buffer = itoa::Buffer::new();
        self.emit_scalar(Scalar {
            tag: None,
            value: buffer.format(v),
            style: ScalarStyle::Plain,
        })
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        let mut buffer = itoa::Buffer::new();
        self.emit_scalar(Scalar {
            tag: None,
            value: buffer.format(v),
            style: ScalarStyle::Plain,
        })
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        let mut buffer = ryu::Buffer::new();
        self.emit_scalar(Scalar {
            tag: None,
            value: buffer.format(v),
            style: ScalarStyle::Plain,
        })
    }

    fn serialize_char(self, v: char) -> Result<()> {
        let mut b = [0u8; 4];
        self.serialize_str(v.encode_utf8(&mut b))
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.emit_scalar(Scalar {
            tag: None,
            value: v,
            style: if crate::de::ambiguous_string(v) {
                ScalarStyle::SingleQuoted
            } else {
                ScalarStyle::Plain
            },
        })
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        use serde::ser::SerializeSeq;
        let mut seq = self.serialize_seq(Some(v.len()))?;
        for byte in v {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }

    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        self.emit_scalar(Scalar {
            tag: None,
            value: "null",
            style: ScalarStyle::Plain,
        })
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        if self.config.tag_unit_variants {
            self.state = State::FoundTag(format!("!{}", variant));
            self.serialize_unit()
        } else {
            self.serialize_str(variant)
        }
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.emit_mapping_start()?;
        self.serialize_str(variant)?;
        value.serialize(&mut *self)?;
        self.emit_mapping_end()
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.emit_sequence_start()?;
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(_len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(_len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.emit_mapping_start()?;
        self.serialize_str(variant)?;
        self.emit_sequence_start()?;
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.emit_mapping_start()?;
        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(_len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.emit_mapping_start()?;
        self.serialize_str(variant)?;
        self.emit_mapping_start()?;
        Ok(self)
    }

    fn collect_str<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Display,
    {
        let string = value.to_string();
        self.serialize_str(&string)
    }
}

impl<'a, W> ser::SerializeSeq for &mut Serializer<'a, W>
where
    W: io::Write + 'a,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, elem: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        elem.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_sequence_end()
    }
}

impl<'a, W> ser::SerializeTuple for &mut Serializer<'a, W>
where
    W: io::Write + 'a,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, elem: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        elem.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_sequence_end()
    }
}

impl<'a, W> ser::SerializeTupleStruct for &mut Serializer<'a, W>
where
    W: io::Write + 'a,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(&mut self, value: &V) -> Result<()>
    where
        V: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_sequence_end()
    }
}

impl<'a, W> ser::SerializeTupleVariant for &mut Serializer<'a, W>
where
    W: io::Write + 'a,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(&mut self, v: &V) -> Result<()>
    where
        V: ?Sized + ser::Serialize,
    {
        v.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_sequence_end()?;
        self.emit_mapping_end()
    }
}

impl<'a, W> ser::SerializeMap for &mut Serializer<'a, W>
where
    W: io::Write + 'a,
{
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.flush_mapping_start()?;
        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn serialize_entry<K, V>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<()>
    where
        K: ?Sized + ser::Serialize,
        V: ?Sized + ser::Serialize,
    {
        key.serialize(&mut **self)?;
        let tagged = matches!(self.state, State::FoundTag(_));
        value.serialize(&mut **self)?;
        if tagged {
            self.state = State::AlreadyTagged;
        }
        Ok(())
    }

    fn end(self) -> Result<()> {
        if let State::CheckForTag = self.state {
            self.emit_mapping_start()?;
        }
        if !matches!(self.state, State::AlreadyTagged) {
            self.emit_mapping_end()?;
        }
        self.state = State::NothingInParticular;
        Ok(())
    }
}

impl<'a, W> ser::SerializeStruct for &mut Serializer<'a, W>
where
    W: io::Write + 'a,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(
        &mut self,
        key: &'static str,
        value: &V,
    ) -> Result<()>
    where
        V: ?Sized + ser::Serialize,
    {
        ser::Serializer::serialize_str(&mut **self, key)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_mapping_end()
    }
}

impl<'a, W> ser::SerializeStructVariant for &mut Serializer<'a, W>
where
    W: io::Write + 'a,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(
        &mut self,
        field: &'static str,
        v: &V,
    ) -> Result<()>
    where
        V: ?Sized + ser::Serialize,
    {
        ser::Serializer::serialize_str(&mut **self, field)?;
        v.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_mapping_end()
    }
}

/// Serialize the given data structure as YAML into the IO stream.
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// return an error.
pub fn to_writer<'a, W, T>(writer: W, value: &T) -> Result<()>
where
    W: io::Write + 'a,
    T: ?Sized + ser::Serialize,
{
    let mut serializer = Serializer::new(writer);
    value.serialize(&mut serializer)
}

/// Serialize the given data structure as a String of YAML.
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// return an error.
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: ?Sized + ser::Serialize,
{
    let mut vec = Vec::with_capacity(128);
    to_writer(&mut vec, value)?;
    String::from_utf8(vec)
        .map_err(|error| Error::new(ErrorImpl::FromUtf8(error)))
}
