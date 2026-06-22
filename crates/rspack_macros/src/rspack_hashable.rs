use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::{
  Attribute, Data, DeriveInput, Error, Fields, Generics, Ident, Index, Path, Result, Type,
  TypeParamBound, parse_quote,
};

#[derive(Default)]
struct TypeOptions {
  hash_crate: Option<Path>,
  json: bool,
}

#[derive(Default)]
struct FieldOptions {
  skip: bool,
  order: Option<usize>,
  null_if_none: bool,
}

pub fn expand_rspack_hashable_derive(input: DeriveInput) -> Result<TokenStream> {
  let options = type_options(&input.attrs)?;
  let hash_crate = options
    .hash_crate
    .unwrap_or_else(|| parse_quote!(::rspack_hash));
  let use_json = options.json;
  let body = if use_json {
    quote! {
      #hash_crate::hash_by_json(self, state);
    }
  } else {
    match &input.data {
      Data::Struct(data) => hash_fields(&hash_crate, &data.fields)?,
      Data::Enum(data) => {
        let arms = data
          .variants
          .iter()
          .map(|variant| hash_variant(&hash_crate, &input.ident, variant))
          .collect::<Result<Vec<_>>>()?;
        quote! {
          match self {
            #(#arms)*
          }
        }
      }
      Data::Union(data) => {
        return Err(Error::new_spanned(
          data.union_token,
          "RspackHashable cannot be derived for unions",
        ));
      }
    }
  };

  let ident = &input.ident;
  let mut generics = input.generics.clone();
  add_rspack_hashable_bounds(&mut generics, &hash_crate, use_json);
  let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

  Ok(quote! {
    impl #impl_generics #hash_crate::RspackHashable for #ident #ty_generics #where_clause {
      fn hash(&self, state: &mut #hash_crate::RspackHash) {
        #body
      }
    }
  })
}

fn type_options(attrs: &[Attribute]) -> Result<TypeOptions> {
  let mut options = TypeOptions::default();
  for attr in attrs {
    if !attr.path().is_ident("rspack_hash") {
      continue;
    }
    attr.parse_nested_meta(|meta| {
      if meta.path.is_ident("crate") {
        let value = meta.value()?;
        options.hash_crate = Some(value.parse::<Path>()?);
        Ok(())
      } else if meta.path.is_ident("json") {
        options.json = true;
        Ok(())
      } else {
        Err(meta.error("unsupported rspack_hash attribute"))
      }
    })?;
  }
  Ok(options)
}

fn field_options(attrs: &[syn::Attribute]) -> Result<FieldOptions> {
  let mut options = FieldOptions::default();
  for attr in attrs {
    if !attr.path().is_ident("rspack_hash") {
      continue;
    }
    attr.parse_nested_meta(|meta| {
      if meta.path.is_ident("skip") {
        options.skip = true;
        Ok(())
      } else if meta.path.is_ident("order") {
        let value = meta.value()?;
        let order = value.parse::<syn::LitInt>()?;
        options.order = Some(order.base10_parse()?);
        Ok(())
      } else if meta.path.is_ident("null_if_none") {
        options.null_if_none = true;
        Ok(())
      } else {
        Err(meta.error("unsupported rspack_hash field attribute"))
      }
    })?;
  }
  Ok(options)
}

fn hash_fields(hash_crate: &Path, fields: &Fields) -> Result<TokenStream> {
  match fields {
    Fields::Named(fields) => {
      let fields = fields
        .named
        .iter()
        .enumerate()
        .map(|(index, field)| {
          let options = field_options(&field.attrs)?;
          if options.skip {
            return Ok(None);
          }
          let Some(ident) = field.ident.as_ref() else {
            return Ok(None);
          };
          let field_name = ident.to_string();
          let hash = hash_field(
            hash_crate,
            &quote! {
              &self.#ident
            },
            &field_name,
            &field.ty,
            &options,
          );
          Ok(Some((options.order.unwrap_or(index), hash)))
        })
        .collect::<Result<Vec<_>>>()?;
      let mut fields = fields.into_iter().flatten().collect::<Vec<_>>();
      sort_fields(&mut fields)?;
      let fields = fields.into_iter().map(|(_, hash)| hash);
      Ok(quote! {
        state.write(b"{");
        let mut is_first_rspack_hash_field = true;
        #(#fields)*
        let _ = is_first_rspack_hash_field;
        state.write(b"}");
      })
    }
    Fields::Unnamed(fields) => {
      let fields = fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(index, field)| {
          let options = field_options(&field.attrs)?;
          if options.skip {
            return Ok(None);
          }
          let field_index = Index::from(index);
          let field_name = index.to_string();
          let hash = hash_field(
            hash_crate,
            &quote! {
              &self.#field_index
            },
            &field_name,
            &field.ty,
            &options,
          );
          Ok(Some((options.order.unwrap_or(index), hash)))
        })
        .collect::<Result<Vec<_>>>()?;
      let mut fields = fields.into_iter().flatten().collect::<Vec<_>>();
      sort_fields(&mut fields)?;
      let fields = fields.into_iter().map(|(_, hash)| hash);
      Ok(quote! {
        state.write(b"{");
        let mut is_first_rspack_hash_field = true;
        #(#fields)*
        let _ = is_first_rspack_hash_field;
        state.write(b"}");
      })
    }
    Fields::Unit => Ok(quote! {
      state.write(b"{}");
    }),
  }
}

fn hash_variant(
  hash_crate: &Path,
  enum_ident: &Ident,
  variant: &syn::Variant,
) -> Result<TokenStream> {
  let variant_ident = &variant.ident;
  let variant_name = variant_ident.to_string();
  match &variant.fields {
    Fields::Named(fields) => {
      let field_idents = fields
        .named
        .iter()
        .map(|field| {
          field
            .ident
            .clone()
            .ok_or_else(|| Error::new_spanned(field, "expected named field"))
        })
        .collect::<Result<Vec<_>>>()?;
      let hashes = fields
        .named
        .iter()
        .zip(&field_idents)
        .enumerate()
        .map(|(index, (field, ident))| {
          let options = field_options(&field.attrs)?;
          if options.skip {
            return Ok(None);
          }
          let field_name = ident.to_string();
          let hash = hash_field(
            hash_crate,
            &quote! { #ident },
            &field_name,
            &field.ty,
            &options,
          );
          Ok(Some((options.order.unwrap_or(index), hash)))
        })
        .collect::<Result<Vec<_>>>()?;
      let mut hashes = hashes.into_iter().flatten().collect::<Vec<_>>();
      sort_fields(&mut hashes)?;
      let hashes = hashes.into_iter().map(|(_, hash)| hash);
      let variant_start = Literal::byte_string(format!("{variant_name}{{").as_bytes());
      Ok(quote! {
        #enum_ident::#variant_ident { #(#field_idents),* } => {
          state.write(#variant_start);
          let mut is_first_rspack_hash_field = true;
          #(#hashes)*
          let _ = is_first_rspack_hash_field;
          state.write(b"}");
        }
      })
    }
    Fields::Unnamed(fields) => {
      let bindings = (0..fields.unnamed.len())
        .map(|index| Ident::new(&format!("field_{index}"), variant_ident.span()))
        .collect::<Vec<_>>();
      let hashes = fields
        .unnamed
        .iter()
        .zip(&bindings)
        .enumerate()
        .map(|(index, (field, ident))| {
          let options = field_options(&field.attrs)?;
          if options.skip {
            return Ok(None);
          }
          let field_name = index.to_string();
          let hash = hash_field(
            hash_crate,
            &quote! { #ident },
            &field_name,
            &field.ty,
            &options,
          );
          Ok(Some((options.order.unwrap_or(index), hash)))
        })
        .collect::<Result<Vec<_>>>()?;
      let mut hashes = hashes.into_iter().flatten().collect::<Vec<_>>();
      sort_fields(&mut hashes)?;
      let hashes = hashes.into_iter().map(|(_, hash)| hash);
      let variant_start = Literal::byte_string(format!("{variant_name}{{").as_bytes());
      Ok(quote! {
        #enum_ident::#variant_ident(#(#bindings),*) => {
          state.write(#variant_start);
          let mut is_first_rspack_hash_field = true;
          #(#hashes)*
          let _ = is_first_rspack_hash_field;
          state.write(b"}");
        }
      })
    }
    Fields::Unit => {
      let variant_name = Literal::byte_string(variant_name.as_bytes());
      Ok(quote! {
        #enum_ident::#variant_ident => {
          state.write(#variant_name);
        }
      })
    }
  }
}

fn hash_field(
  hash_crate: &Path,
  target: &TokenStream,
  field_name: &str,
  ty: &Type,
  options: &FieldOptions,
) -> TokenStream {
  let first_key = Literal::byte_string(format!("{field_name}:").as_bytes());
  let next_key = Literal::byte_string(format!(",{field_name}:").as_bytes());
  let hash_key = quote! {
    if is_first_rspack_hash_field {
      state.write(#first_key);
    } else {
      state.write(#next_key);
    }
    is_first_rspack_hash_field = false;
  };
  if options.null_if_none {
    quote! {
      match #target {
        Some(value) => {
          #hash_key
          #hash_crate::RspackHashable::hash(value, state);
        }
        None => {
          #hash_key
          state.write(b"null");
        }
      }
    }
  } else if is_option_type(ty) {
    quote! {
      if let Some(value) = #target {
        #hash_key
        #hash_crate::RspackHashable::hash(value, state);
      }
    }
  } else {
    quote! {
      #hash_key
      #hash_crate::RspackHashable::hash(#target, state);
    }
  }
}

fn is_option_type(ty: &Type) -> bool {
  let Type::Path(path) = ty else {
    return false;
  };
  path
    .path
    .segments
    .last()
    .is_some_and(|segment| segment.ident == "Option")
}

fn sort_fields(fields: &mut [(usize, TokenStream)]) -> Result<()> {
  fields.sort_by_key(|(order, _)| *order);
  for window in fields.windows(2) {
    if window[0].0 == window[1].0 {
      return Err(Error::new_spanned(
        &window[1].1,
        "duplicate rspack_hash field order",
      ));
    }
  }
  Ok(())
}

fn add_rspack_hashable_bounds(generics: &mut Generics, hash_crate: &Path, json_hash: bool) {
  for param in generics.type_params_mut() {
    let bound: TypeParamBound = parse_quote!(#hash_crate::RspackHashable);
    param.bounds.push(bound);
    if json_hash {
      let serde_bound: TypeParamBound = parse_quote!(::serde::Serialize);
      param.bounds.push(serde_bound);
    }
  }
}
