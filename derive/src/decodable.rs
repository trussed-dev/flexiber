use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Attribute, DataStruct, Field, Ident};
use synstructure::Structure;

use crate::{extract_attrs_optional_tag, FieldAttrs};

/// Derive Decodable on a struct
pub(crate) struct DeriveDecodableStruct {
    /// Field decoders
    decode_fields: TokenStream,

    /// Bound fields of a struct to be returned
    decode_result: TokenStream,
}

impl DeriveDecodableStruct {
    pub fn derive(s: Structure<'_>, data: &DataStruct, name: &Ident, attrs: &[Attribute]) -> TokenStream {

        let (tag, _) = extract_attrs_optional_tag(name, attrs);

        let mut state = Self {
            decode_fields: TokenStream::new(),
            decode_result: TokenStream::new(),
        };

        for field in &data.fields {
            state.derive_field(field);
        }

        state.finish(&s, tag)
    }

    /// Derive handling for a particular `#[field(...)]`
    fn derive_field(&mut self, field: &Field) {
        let attrs = FieldAttrs::new(field);
        self.derive_field_decoder(&attrs);
    }

    /// Derive code for decoding a field of a message
    fn derive_field_decoder(&mut self, field: &FieldAttrs) {
        let field_name = &field.name;
        let field_tag = field.tag;
        let field_decoder = if field.slice {
            quote! { let #field_name =
                decoder.decode_tagged_slice(::simple_tlv::Tag::try_from(#field_tag).unwrap())?.try_into()
                    .map_err(|_| simple_tlv::ErrorKind::Length { tag: simple_tlv::Tag::try_from(#field_tag).unwrap() })?;
            }
        } else {
            quote! { let #field_name = decoder.decode_tagged_value(::simple_tlv::Tag::try_from(#field_tag).unwrap())?; }
        };
        field_decoder.to_tokens(&mut self.decode_fields);

        let field_result = quote!(#field_name,);
        field_result.to_tokens(&mut self.decode_result);
    }

    /// Finish deriving a struct
    fn finish(self, s: &Structure<'_>, tag: Option<u8>) -> TokenStream {

        let decode_fields = self.decode_fields;
        let decode_result = self.decode_result;

        if let Some(tag) = tag {
            s.gen_impl(quote! {
                gen impl<'a> core::convert::TryFrom<simple_tlv::TaggedSlice<'a>> for @Self {
                    type Error = simple_tlv::Error;

                    fn try_from(tagged_slice: simple_tlv::TaggedSlice<'a>) -> simple_tlv::Result<Self> {
                        use core::convert::TryInto;
                        tagged_slice.tag().assert_eq(simple_tlv::Tag::try_from(#tag).unwrap())?;
                        tagged_slice.decode_nested(|decoder| {
                            #decode_fields

                            Ok(Self { #decode_result })
                        })
                    }
                }
            })
        } else {
            s.gen_impl(quote! {
                gen impl<'a> simple_tlv::Decodable<'a> for @Self {
                    fn decode(decoder: &mut simple_tlv::Decoder<'a>) -> simple_tlv::Result<Self> {
                        use core::convert::{TryFrom, TryInto};
                        #decode_fields
                        Ok(Self { #decode_result })
                    }
                }
            })
        }
    }
}

