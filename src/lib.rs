//! This crate provides the `json!` macro. It lets you write JSON literal inside your Rust code.
//! Inside the macro, you can also write arbitrary Rust expressions in place of JSON value,
//! as long as they don't include JSON delimiter(`'` or `:`).
//! You need `cc-traits` in your dependency to use this macro.
//!
//! ## Example
//! ```
//! # use inline_json::json;
//! use json_trait::{ForeignMutableJson, BuildableJson};
//! # fn test<T: ForeignMutableJson + BuildableJson>() {
//! let json: T = json!(T, {"name": "example", "array": ["foo", "bar"]});
//! // Expands to:
//! assert_eq!(json, {
//!     let mut object = <T>::empty_object();
//!     <_ as cc_traits::MapInsert<_>>::insert(&mut object, "name".to_string(), <_ as Into<T>>::into("example"));
//!     <_ as cc_traits::MapInsert<_>>::insert(&mut object, "array".to_string(), {
//!         let mut array = <T>::empty_array();
//!         <_ as cc_traits::PushBack>::push_back(&mut array, <_ as Into<T>>::into("foo"));
//!         <_ as cc_traits::PushBack>::push_back(&mut array, <_ as Into<T>>::into("bar"));
//!         array.into()
//!     });
//!     object.into()
//! });
//! # }
//! # test::<serde_json::Value>();
//! ```

extern crate proc_macro;

use proc_macro::{Delimiter, TokenStream, TokenTree};
use quote::quote;
use syn::{parse_macro_input, Expr, Type};

#[proc_macro]
pub fn json(json: TokenStream) -> TokenStream {
	let mut iter = json.into_iter().peekable();
	let ty = iter.by_ref().take_while(is_not_punct(',')).collect();
	let ty = parse_macro_input!(ty);
	json_impl(&ty, Box::new(iter))
}

fn json_impl<'a>(ty: &Type, iter: Box<dyn Iterator<Item = TokenTree> + 'a>) -> TokenStream {
	let mut iter = iter.peekable();
	if let Some(TokenTree::Group(group)) = iter.peek() {
		match group.delimiter() {
			Delimiter::Brace => {
				let mut iter = group.stream().into_iter().peekable();
				let mut entries = Vec::new();
				while iter.peek().is_some() {
					let key = iter.by_ref().take_while(is_not_punct(':')).collect();
					let key = parse_macro_input!(key as Expr);
					let value: proc_macro2::TokenStream = json_impl(ty, Box::new(iter.by_ref().take_while(is_not_punct(',')))).into();
					entries.push(quote! { <_ as cc_traits::MapInsert<_>>::insert(&mut object, (#key).to_string(), #value); });
				}
				return (quote! {{
					let mut object = <#ty>::empty_object();
					#(#entries)*
					object.into()
				}})
				.into();
			}
			Delimiter::Bracket => {
				let mut iter = group.stream().into_iter().peekable();
				let mut values = Vec::<proc_macro2::TokenStream>::new();
				while iter.peek().is_some() {
					values.push(json_impl(ty, Box::new(iter.by_ref().take_while(is_not_punct(',')))).into());
				}
				return (quote! {{
					let mut array = <#ty>::empty_array();
					#(<_ as cc_traits::PushBack>::push_back(&mut array, #values);)*
					array.into()
				}})
				.into();
			}
			_ => {}
		}
	}
	let expr = iter.collect();
	let expr = parse_macro_input!(expr as Expr);
	(quote! { <_ as Into<#ty>>::into(#expr) }).into()
}

fn is_not_punct(ch: char) -> impl Fn(&TokenTree) -> bool {
	move |tt| {
		if let TokenTree::Punct(punct) = tt {
			punct != &ch
		} else {
			true
		}
	}
}
