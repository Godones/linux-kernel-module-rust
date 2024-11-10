use proc_macro2::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn domain_main(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = TokenStream::from(item);
    let panic = panic_impl();
    quote! (
        #[no_mangle]
        #item
        #panic
    )
    .into()
}

fn panic_impl() -> TokenStream {
    quote!(
        #[panic_handler]
        fn panic(info: &PanicInfo) -> ! {
            if let Some(p) = info.location() {
                basic::console::println_color!(
                    31,
                    "line {}, file {}: {}",
                    p.line(),
                    p.file(),
                    info.message()
                );
            } else {
                basic::console::println_color!(31, "no location information available");
            }
            basic::backtrace(domain_id());
            static FAKE_LOCK: basic::sync::Mutex<()> = basic::sync::Mutex::new(());
            #[cfg(feature = "rust-unwind")]
            {
                let _guard = FAKE_LOCK.lock();
                basic::unwind_from_panic();
            }
            loop {}
        }
    )
    .into()
}
