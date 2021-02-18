use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Attribute, DataStruct, Field, Ident};
use synstructure::Structure;

use crate::{extract_attrs_optional_tag, FieldAttrs};

/// Derive Encodable on a struct
pub(crate) struct DeriveEncodableStruct {
    /// Fields of a struct to be serialized
    encode_fields: TokenStream,
}

impl DeriveEncodableStruct {
    pub fn derive(s: Structure<'_>, data: &DataStruct, name: &Ident, attrs: &[Attribute]) -> TokenStream {

        let (tag, _) = extract_attrs_optional_tag(name, attrs);

        let mut state = Self {
            encode_fields: TokenStream::new(),
        };

        for field in &data.fields {
            state.derive_field(field);
        }

        state.finish(&s, tag)
    }

    /// Derive handling for a particular `#[field(...)]`
    fn derive_field(&mut self, field: &Field) {
        let attrs = FieldAttrs::new(field);
        self.derive_field_encoder(&attrs);
    }

    /// Derive code for encoding a field of a message
    fn derive_field_encoder(&mut self, field: &FieldAttrs) {
        let field_name = &field.name;
        let field_tag = field.tag;
        let field_encoder = if field.slice {
            quote! { &(::simple_tlv::TaggedSlice::from(simple_tlv::Tag::try_from(#field_tag).unwrap(), &self.#field_name)?), }
        } else {
            quote! { &(::simple_tlv::Tag::try_from(#field_tag).unwrap().with_value(&self.#field_name)), }
        };
        field_encoder.to_tokens(&mut self.encode_fields);
    }

    /// Finish deriving a struct
    fn finish(self, s: &Structure<'_>, tag: Option<u8>) -> TokenStream {


        let encode_fields = self.encode_fields;

        if let Some(tag) = tag {
            s.gen_impl(quote! {
                gen impl simple_tlv::Tagged for @Self {
                    fn tag() -> simple_tlv::Tag {
                        // TODO(nickray): FIXME FIXME
                        use core::convert::TryFrom;
                        simple_tlv::Tag::try_from(#tag).unwrap()
                    }
                }

                gen impl simple_tlv::Container for @Self {
                    fn fields<F, T>(&self, field_encoder: F) -> simple_tlv::Result<T>
                    where
                        F: FnOnce(&[&dyn simple_tlv::Encodable]) -> simple_tlv::Result<T>,
                    {
                        use core::convert::TryFrom;
                        field_encoder(&[#encode_fields])
                    }
                }
            })
        } else {
            s.gen_impl(quote! {
                gen impl simple_tlv::Container for @Self {
                    fn fields<F, T>(&self, field_encoder: F) -> simple_tlv::Result<T>
                    where
                        F: FnOnce(&[&dyn simple_tlv::Encodable]) -> simple_tlv::Result<T>,
                    {
                        use core::convert::TryFrom;
                        field_encoder(&[#encode_fields])
                    }
                }

                gen impl simple_tlv::Encodable for @Self {
                    fn encoded_length(&self) -> simple_tlv::Result<simple_tlv::Length> {
                        use core::convert::TryFrom;
                        use simple_tlv::Container;
                        self.fields(|encodables| simple_tlv::Length::try_from(encodables))
                    }

                    fn encode(&self, encoder: &mut simple_tlv::Encoder<'_>) -> simple_tlv::Result<()> {
                        use simple_tlv::Container;
                        self.fields(|fields| encoder.encode_untagged_collection(fields))
                    }
                }
            })
        }
    }
}

