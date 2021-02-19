use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Attribute, DataStruct, Field, Ident};
use synstructure::Structure;

use crate::{extract_attrs_optional_tag, FieldAttrs, Tag};

/// Derive Encodable on a struct
pub(crate) struct DeriveEncodableStruct {
    /// Fields of a struct to be serialized
    encode_fields: TokenStream,
}

impl DeriveEncodableStruct {
    pub fn derive(s: Structure<'_>, data: &DataStruct, name: &Ident, attrs: &[Attribute]) -> TokenStream {

        let (tag, _slice) = extract_attrs_optional_tag(name, attrs);

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
        let tag = field.tag;

        let class = tag.class as u8;
        let constructed = tag.constructed;
        let tag_number = tag.number;

        let field_encoder = if field.slice {
            quote! { &(::flexiber::TaggedSlice::from(flexiber::Tag::from(flexiber::Class::try_from(#class).unwrap(), #constructed, #tag_number), &self.#field_name)?), }
        } else {
            quote! { &(::flexiber::Tag::from(flexiber::Class::try_from(#class).unwrap(), #constructed, #tag_number).with_value(&self.#field_name)), }
        };
        field_encoder.to_tokens(&mut self.encode_fields);
    }

    /// Finish deriving a struct
    fn finish(self, s: &Structure<'_>, tag: Option<Tag>) -> TokenStream {


        let encode_fields = self.encode_fields;

        if let Some(tag) = tag {
            let class = tag.class as u8;
            let constructed = tag.constructed;
            let tag_number = tag.number;
            s.gen_impl(quote! {
                gen impl flexiber::Tagged for @Self {
                    fn tag() -> flexiber::Tag {
                        // TODO(nickray): FIXME FIXME
                        use core::convert::TryFrom;
                        flexiber::Tag::from(flexiber::Class::try_from(#class).unwrap(), #constructed, #tag_number)
                    }
                }

                gen impl flexiber::Container for @Self {
                    fn fields<F, T>(&self, field_encoder: F) -> flexiber::Result<T>
                    where
                        F: FnOnce(&[&dyn flexiber::Encodable]) -> flexiber::Result<T>,
                    {
                        use core::convert::TryFrom;
                        field_encoder(&[#encode_fields])
                    }
                }
            })
        } else {
            s.gen_impl(quote! {
                gen impl flexiber::Container for @Self {
                    fn fields<F, T>(&self, field_encoder: F) -> flexiber::Result<T>
                    where
                        F: FnOnce(&[&dyn flexiber::Encodable]) -> flexiber::Result<T>,
                    {
                        use core::convert::TryFrom;
                        field_encoder(&[#encode_fields])
                    }
                }

                gen impl flexiber::Encodable for @Self {
                    fn encoded_length(&self) -> flexiber::Result<flexiber::Length> {
                        use core::convert::TryFrom;
                        use flexiber::Container;
                        self.fields(|encodables| flexiber::Length::try_from(encodables))
                    }

                    fn encode(&self, encoder: &mut flexiber::Encoder<'_>) -> flexiber::Result<()> {
                        use flexiber::Container;
                        self.fields(|fields| encoder.encode_untagged_collection(fields))
                    }
                }
            })
        }
    }
}

