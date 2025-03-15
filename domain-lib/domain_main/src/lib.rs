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
        #[global_allocator]
        static HEAP_ALLOCATOR: malloc::HeapAllocator =  malloc::HeapAllocator::new(corelib::alloc_raw_pages);
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
            let mut time = 0;
            #[cfg(feature = "rust-unwind-time")]
            {
                time = corelib::kernel::time::ktime_get_ns();
            }
            // basic::println_color!(31, "[time] {}ns", time);
            // basic::println_color!(31, "{:?}: {:?}",info.location(), info.message());
            basic::backtrace(domain_id());
            #[cfg(feature = "rust-unwind")]
            {
                basic::unwind_from_panic();
            }
            #[cfg(feature = "rust-unwind-time")]
            {
                basic::unwind_from_panic_with_time(time);
            }
            loop {}
        }
    )
    .into()
}
