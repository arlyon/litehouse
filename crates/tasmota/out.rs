name_interface: "wasi:io/poll@0.2.0-rc-2023-11-10"
name_interface: "wasi:clocks/monotonic-clock@0.2.0-rc-2023-11-10"
name_interface: "wasi:io/error@0.2.0-rc-2023-11-10"
name_interface: "wasi:io/streams@0.2.0-rc-2023-11-10"
name_interface: "wasi:http/types@0.2.0-rc-2023-12-05"
name_interface: "wasi:http/outgoing-handler@0.2.0-rc-2023-12-05"
name_interface: "litehouse:plugin/plugin"
name_interface: "litehouse:plugin/plugin"
#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use std::sync::Mutex;
use exports::litehouse::plugin::plugin::Guest;
use wasi::http::{
    outgoing_handler,
    types::{Fields, OutgoingRequest, RequestOptions, Scheme},
};
use crate::exports::litehouse::plugin::plugin::{Every, GuestRunner, Subscription, TimeUnit, Update};
pub type Event = litehouse::plugin::plugin::Event;
#[allow(unused_unsafe, clippy::all)]
pub fn update(event: Event) {
    #[allow(unused_imports)]
    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
    unsafe {
        let litehouse::plugin::plugin::Event {
            id: id0,
            timestamp: timestamp0,
            inner: inner0,
        } = event;
        use litehouse::plugin::plugin::Update as V1;
        let (result2_0, result2_1) = match inner0 {
            V1::Time(e) => (0i32, plugin::wit_bindgen::rt::as_i64(e)),
            V1::Temperature(e) => (1i32, (plugin::wit_bindgen::rt::as_f64(e)).to_bits() as i64),
            V1::WindSpeed(e) => (2i32, (plugin::wit_bindgen::rt::as_f64(e)).to_bits() as i64),
        };
        #[cfg(target_arch = "wasm32")]
        #[link(wasm_import_module = "$root")]
        extern "C" {
            #[link_name = "update"]
            fn wit_import(_: i64, _: i64, _: i32, _: i64);
        }
        wit_import(
            plugin::wit_bindgen::rt::as_i64(id0),
            plugin::wit_bindgen::rt::as_i64(timestamp0),
            result2_0,
            result2_1,
        );
    }
}
pub mod litehouse {
    pub mod plugin {
        #[allow(clippy::all)]
        pub mod plugin {
            #[used]
            #[doc(hidden)]
            #[cfg(target_arch = "wasm32")]
            static __FORCE_SECTION_REF: fn() = super::super::super::__link_section;
            pub enum Update {
                Time(u64),
                Temperature(f64),
                WindSpeed(f64),
            }
            #[automatically_derived]
            impl ::core::clone::Clone for Update {
                #[inline]
                fn clone(&self) -> Update {
                    let _: ::core::clone::AssertParamIsClone<u64>;
                    let _: ::core::clone::AssertParamIsClone<f64>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for Update {}
            impl ::core::fmt::Debug for Update {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    match self {
                        Update::Time(e) => f.debug_tuple("Update::Time").field(e).finish(),
                        Update::Temperature(e) => {
                            f.debug_tuple("Update::Temperature").field(e).finish()
                        }
                        Update::WindSpeed(e) => {
                            f.debug_tuple("Update::WindSpeed").field(e).finish()
                        }
                    }
                }
            }
            #[repr(C)]
            pub struct Event {
                pub id: u64,
                pub timestamp: u64,
                pub inner: Update,
            }
            #[automatically_derived]
            impl ::core::clone::Clone for Event {
                #[inline]
                fn clone(&self) -> Event {
                    let _: ::core::clone::AssertParamIsClone<u64>;
                    let _: ::core::clone::AssertParamIsClone<Update>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for Event {}
            impl ::core::fmt::Debug for Event {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    f.debug_struct("Event")
                        .field("id", &self.id)
                        .field("timestamp", &self.timestamp)
                        .field("inner", &self.inner)
                        .finish()
                }
            }
            pub enum TimeUnit {
                Second,
                Minute,
                Hour,
                Day,
                Week,
                Month,
                Year,
            }
            #[automatically_derived]
            impl ::core::clone::Clone for TimeUnit {
                #[inline]
                fn clone(&self) -> TimeUnit {
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for TimeUnit {}
            impl ::core::fmt::Debug for TimeUnit {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    match self {
                        TimeUnit::Second => f.debug_tuple("TimeUnit::Second").finish(),
                        TimeUnit::Minute => f.debug_tuple("TimeUnit::Minute").finish(),
                        TimeUnit::Hour => f.debug_tuple("TimeUnit::Hour").finish(),
                        TimeUnit::Day => f.debug_tuple("TimeUnit::Day").finish(),
                        TimeUnit::Week => f.debug_tuple("TimeUnit::Week").finish(),
                        TimeUnit::Month => f.debug_tuple("TimeUnit::Month").finish(),
                        TimeUnit::Year => f.debug_tuple("TimeUnit::Year").finish(),
                    }
                }
            }
            #[repr(C)]
            pub struct Every {
                pub amount: u64,
                pub unit: TimeUnit,
            }
            #[automatically_derived]
            impl ::core::clone::Clone for Every {
                #[inline]
                fn clone(&self) -> Every {
                    let _: ::core::clone::AssertParamIsClone<u64>;
                    let _: ::core::clone::AssertParamIsClone<TimeUnit>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for Every {}
            impl ::core::fmt::Debug for Every {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    f.debug_struct("Every")
                        .field("amount", &self.amount)
                        .field("unit", &self.unit)
                        .finish()
                }
            }
            pub enum TimeSubscription {
                Every(Every),
                At(u64),
            }
            #[automatically_derived]
            impl ::core::clone::Clone for TimeSubscription {
                #[inline]
                fn clone(&self) -> TimeSubscription {
                    let _: ::core::clone::AssertParamIsClone<Every>;
                    let _: ::core::clone::AssertParamIsClone<u64>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for TimeSubscription {}
            impl ::core::fmt::Debug for TimeSubscription {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    match self {
                        TimeSubscription::Every(e) => {
                            f.debug_tuple("TimeSubscription::Every").field(e).finish()
                        }
                        TimeSubscription::At(e) => {
                            f.debug_tuple("TimeSubscription::At").field(e).finish()
                        }
                    }
                }
            }
            pub enum Subscription {
                Time(TimeSubscription),
            }
            #[automatically_derived]
            impl ::core::clone::Clone for Subscription {
                #[inline]
                fn clone(&self) -> Subscription {
                    let _: ::core::clone::AssertParamIsClone<TimeSubscription>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for Subscription {}
            impl ::core::fmt::Debug for Subscription {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    match self {
                        Subscription::Time(e) => {
                            f.debug_tuple("Subscription::Time").field(e).finish()
                        }
                    }
                }
            }
            #[repr(transparent)]
            pub struct Runner {
                handle: plugin::wit_bindgen::rt::Resource<Runner>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for Runner {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "Runner",
                        "handle",
                        &&self.handle,
                    )
                }
            }
            impl Runner {
                #[doc(hidden)]
                pub unsafe fn from_handle(handle: u32) -> Self {
                    Self {
                        handle: plugin::wit_bindgen::rt::Resource::from_handle(handle),
                    }
                }
                #[doc(hidden)]
                pub fn into_handle(self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::into_handle(self.handle)
                }
                #[doc(hidden)]
                pub fn handle(&self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::handle(&self.handle)
                }
            }
            unsafe impl plugin::wit_bindgen::rt::WasmResource for Runner {
                #[inline]
                unsafe fn drop(_handle: u32) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        #[link(wasm_import_module = "litehouse:plugin/plugin")]
                        extern "C" {
                            #[link_name = "[resource-drop]runner"]
                            fn drop(_: u32);
                        }
                        drop(_handle);
                    }
                }
            }
            impl Runner {
                #[allow(unused_unsafe, clippy::all)]
                pub fn new() -> Self {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "litehouse:plugin/plugin")]
                        extern "C" {
                            #[link_name = "[constructor]runner"]
                            fn wit_import() -> i32;
                        }
                        let ret = wit_import();
                        Runner::from_handle(ret as u32)
                    }
                }
            }
            impl Runner {
                #[allow(unused_unsafe, clippy::all)]
                pub fn subscribe(
                    &self,
                ) -> Result<plugin::wit_bindgen::rt::vec::Vec<Subscription>, u32> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 12]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "litehouse:plugin/plugin")]
                        extern "C" {
                            #[link_name = "[method]runner.subscribe"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = {
                                    let l2 = *((ptr0 + 4) as *const i32);
                                    let l3 = *((ptr0 + 8) as *const i32);
                                    let base12 = l2;
                                    let len12 = l3;
                                    let mut result12 = Vec::with_capacity(len12 as usize);
                                    for i in 0..len12 {
                                        let base = base12 + i * 32;
                                        let e12 = {
                                            let l4 = i32::from(*((base + 0) as *const u8));
                                            let v11 = match l4 {
                                                n => {
                                                    if true {
                                                        match (&n, &0) {
                                                            (left_val, right_val) => {
                                                                if !(*left_val == *right_val) {
                                                                    let kind = :: core :: panicking :: AssertKind :: Eq ;
                                                                    :: core :: panicking :: assert_failed (kind , & * left_val , & * right_val , :: core :: option :: Option :: Some (format_args ! ("invalid enum discriminant"))) ;
                                                                }
                                                            }
                                                        };
                                                    };
                                                    let e11 = {
                                                        let l5 =
                                                            i32::from(*((base + 8) as *const u8));
                                                        let v10 = match l5 {
                                                            0 => {
                                                                let e10 = {
                                                                    let l6 = *((base + 16)
                                                                        as *const i64);
                                                                    let l7 = i32::from(
                                                                        *((base + 24) as *const u8),
                                                                    );
                                                                    let v8 = match l7 {
                                                                        0 => TimeUnit::Second,
                                                                        1 => TimeUnit::Minute,
                                                                        2 => TimeUnit::Hour,
                                                                        3 => TimeUnit::Day,
                                                                        4 => TimeUnit::Week,
                                                                        5 => TimeUnit::Month,
                                                                        n => {
                                                                            if true {
                                                                                match (& n , & 6) { (left_val , right_val) => { if ! (* left_val == * right_val) { let kind = :: core :: panicking :: AssertKind :: Eq ; :: core :: panicking :: assert_failed (kind , & * left_val , & * right_val , :: core :: option :: Option :: Some (format_args ! ("invalid enum discriminant"))) ; } } } ;
                                                                            };
                                                                            TimeUnit::Year
                                                                        }
                                                                    };
                                                                    Every {
                                                                        amount: l6 as u64,
                                                                        unit: v8,
                                                                    }
                                                                };
                                                                TimeSubscription::Every(e10)
                                                            }
                                                            n => {
                                                                if true {
                                                                    match (&n, &1) {
                                                                        (left_val, right_val) => {
                                                                            if !(*left_val
                                                                                == *right_val)
                                                                            {
                                                                                let kind = :: core :: panicking :: AssertKind :: Eq ;
                                                                                :: core :: panicking :: assert_failed (kind , & * left_val , & * right_val , :: core :: option :: Option :: Some (format_args ! ("invalid enum discriminant"))) ;
                                                                            }
                                                                        }
                                                                    };
                                                                };
                                                                let e10 = {
                                                                    let l9 = *((base + 16)
                                                                        as *const i64);
                                                                    l9 as u64
                                                                };
                                                                TimeSubscription::At(e10)
                                                            }
                                                        };
                                                        v10
                                                    };
                                                    Subscription::Time(e11)
                                                }
                                            };
                                            v11
                                        };
                                        result12.push(e12);
                                    }
                                    plugin::wit_bindgen::rt::dealloc(
                                        base12,
                                        (len12 as usize) * 32,
                                        8,
                                    );
                                    result12
                                };
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l13 = *((ptr0 + 4) as *const i32);
                                    l13 as u32
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl Runner {
                #[allow(unused_unsafe, clippy::all)]
                pub fn update(&self, events: &[Event]) -> Result<bool, u32> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 8]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let vec1 = events;
                        let len1 = vec1.len() as i32;
                        let layout1 = alloc::Layout::from_size_align_unchecked(vec1.len() * 32, 8);
                        let result1 = if layout1.size() != 0 {
                            let ptr = alloc::alloc(layout1);
                            if ptr.is_null() {
                                alloc::handle_alloc_error(layout1);
                            }
                            ptr
                        } else {
                            {
                                ::core::ptr::null_mut()
                            }
                        };
                        for (i, e) in vec1.into_iter().enumerate() {
                            let base = result1 as i32 + (i as i32) * 32;
                            {
                                let Event {
                                    id: id0,
                                    timestamp: timestamp0,
                                    inner: inner0,
                                } = e;
                                *((base + 0) as *mut i64) = plugin::wit_bindgen::rt::as_i64(id0);
                                *((base + 8) as *mut i64) =
                                    plugin::wit_bindgen::rt::as_i64(timestamp0);
                                match inner0 {
                                    Update::Time(e) => {
                                        *((base + 16) as *mut u8) = (0i32) as u8;
                                        *((base + 24) as *mut i64) =
                                            plugin::wit_bindgen::rt::as_i64(e);
                                    }
                                    Update::Temperature(e) => {
                                        *((base + 16) as *mut u8) = (1i32) as u8;
                                        *((base + 24) as *mut f64) =
                                            plugin::wit_bindgen::rt::as_f64(e);
                                    }
                                    Update::WindSpeed(e) => {
                                        *((base + 16) as *mut u8) = (2i32) as u8;
                                        *((base + 24) as *mut f64) =
                                            plugin::wit_bindgen::rt::as_f64(e);
                                    }
                                }
                            }
                        }
                        let ptr2 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "litehouse:plugin/plugin")]
                        extern "C" {
                            #[link_name = "[method]runner.update"]
                            fn wit_import(_: i32, _: i32, _: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, result1 as i32, len1, ptr2);
                        let l3 = i32::from(*((ptr2 + 0) as *const u8));
                        if layout1.size() != 0 {
                            alloc::dealloc(result1, layout1);
                        }
                        match l3 {
                            0 => {
                                let e = {
                                    let l4 = i32::from(*((ptr2 + 4) as *const u8));
                                    plugin::wit_bindgen::rt::bool_lift(l4 as u8)
                                };
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l5 = *((ptr2 + 4) as *const i32);
                                    l5 as u32
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            #[allow(unused_unsafe, clippy::all)]
            pub fn generate_config_schema() -> Option<plugin::wit_bindgen::rt::string::String> {
                #[allow(unused_imports)]
                use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                unsafe {
                    #[repr(align(4))]
                    struct RetArea([u8; 12]);
                    let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                    let ptr0 = ret_area.as_mut_ptr() as i32;
                    #[cfg(target_arch = "wasm32")]
                    #[link(wasm_import_module = "litehouse:plugin/plugin")]
                    extern "C" {
                        #[link_name = "generate-config-schema"]
                        fn wit_import(_: i32);
                    }
                    wit_import(ptr0);
                    let l1 = i32::from(*((ptr0 + 0) as *const u8));
                    match l1 {
                        0 => None,
                        1 => {
                            let e = {
                                let l2 = *((ptr0 + 4) as *const i32);
                                let l3 = *((ptr0 + 8) as *const i32);
                                let len4 = l3 as usize;
                                let bytes4 = Vec::from_raw_parts(l2 as *mut _, len4, len4);
                                plugin::wit_bindgen::rt::string_lift(bytes4)
                            };
                            Some(e)
                        }
                        _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                    }
                }
            }
        }
    }
}
pub mod wasi {
    pub mod clocks {
        #[allow(clippy::all)]
        pub mod monotonic_clock {
            #[used]
            #[doc(hidden)]
            #[cfg(target_arch = "wasm32")]
            static __FORCE_SECTION_REF: fn() = super::super::super::__link_section;
            pub type Pollable = super::super::super::wasi::io::poll::Pollable;
            /// An instant in time, in nanoseconds. An instant is relative to an
            /// unspecified initial value, and can only be compared to instances from
            /// the same monotonic-clock.
            pub type Instant = u64;
            /// A duration of time, in nanoseconds.
            pub type Duration = u64;
            #[allow(unused_unsafe, clippy::all)]
            /// Read the current value of the clock.
            ///
            /// The clock is monotonic, therefore calling this function repeatedly will
            /// produce a sequence of non-decreasing values.
            pub fn now() -> Instant {
                #[allow(unused_imports)]
                use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                unsafe {
                    #[cfg(target_arch = "wasm32")]
                    #[link(wasm_import_module = "wasi:clocks/monotonic-clock@0.2.0-rc-2023-11-10")]
                    extern "C" {
                        #[link_name = "now"]
                        fn wit_import() -> i64;
                    }
                    let ret = wit_import();
                    ret as u64
                }
            }
            #[allow(unused_unsafe, clippy::all)]
            /// Query the resolution of the clock. Returns the duration of time
            /// corresponding to a clock tick.
            pub fn resolution() -> Duration {
                #[allow(unused_imports)]
                use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                unsafe {
                    #[cfg(target_arch = "wasm32")]
                    #[link(wasm_import_module = "wasi:clocks/monotonic-clock@0.2.0-rc-2023-11-10")]
                    extern "C" {
                        #[link_name = "resolution"]
                        fn wit_import() -> i64;
                    }
                    let ret = wit_import();
                    ret as u64
                }
            }
            #[allow(unused_unsafe, clippy::all)]
            /// Create a `pollable` which will resolve once the specified instant
            /// occured.
            pub fn subscribe_instant(when: Instant) -> Pollable {
                #[allow(unused_imports)]
                use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                unsafe {
                    #[cfg(target_arch = "wasm32")]
                    #[link(wasm_import_module = "wasi:clocks/monotonic-clock@0.2.0-rc-2023-11-10")]
                    extern "C" {
                        #[link_name = "subscribe-instant"]
                        fn wit_import(_: i64) -> i32;
                    }
                    let ret = wit_import(plugin::wit_bindgen::rt::as_i64(when));
                    super::super::super::wasi::io::poll::Pollable::from_handle(ret as u32)
                }
            }
            #[allow(unused_unsafe, clippy::all)]
            /// Create a `pollable` which will resolve once the given duration has
            /// elapsed, starting at the time at which this function was called.
            /// occured.
            pub fn subscribe_duration(when: Duration) -> Pollable {
                #[allow(unused_imports)]
                use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                unsafe {
                    #[cfg(target_arch = "wasm32")]
                    #[link(wasm_import_module = "wasi:clocks/monotonic-clock@0.2.0-rc-2023-11-10")]
                    extern "C" {
                        #[link_name = "subscribe-duration"]
                        fn wit_import(_: i64) -> i32;
                    }
                    let ret = wit_import(plugin::wit_bindgen::rt::as_i64(when));
                    super::super::super::wasi::io::poll::Pollable::from_handle(ret as u32)
                }
            }
        }
    }
    pub mod http {
        #[allow(clippy::all)]
        pub mod types {
            #[used]
            #[doc(hidden)]
            #[cfg(target_arch = "wasm32")]
            static __FORCE_SECTION_REF: fn() = super::super::super::__link_section;
            pub type Duration = super::super::super::wasi::clocks::monotonic_clock::Duration;
            pub type InputStream = super::super::super::wasi::io::streams::InputStream;
            pub type OutputStream = super::super::super::wasi::io::streams::OutputStream;
            pub type IoError = super::super::super::wasi::io::error::Error;
            pub type Pollable = super::super::super::wasi::io::poll::Pollable;
            /// This type corresponds to HTTP standard Methods.
            pub enum Method {
                Get,
                Head,
                Post,
                Put,
                Delete,
                Connect,
                Options,
                Trace,
                Patch,
                Other(plugin::wit_bindgen::rt::string::String),
            }
            #[automatically_derived]
            impl ::core::clone::Clone for Method {
                #[inline]
                fn clone(&self) -> Method {
                    match self {
                        Method::Get => Method::Get,
                        Method::Head => Method::Head,
                        Method::Post => Method::Post,
                        Method::Put => Method::Put,
                        Method::Delete => Method::Delete,
                        Method::Connect => Method::Connect,
                        Method::Options => Method::Options,
                        Method::Trace => Method::Trace,
                        Method::Patch => Method::Patch,
                        Method::Other(__self_0) => {
                            Method::Other(::core::clone::Clone::clone(__self_0))
                        }
                    }
                }
            }
            impl ::core::fmt::Debug for Method {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    match self {
                        Method::Get => f.debug_tuple("Method::Get").finish(),
                        Method::Head => f.debug_tuple("Method::Head").finish(),
                        Method::Post => f.debug_tuple("Method::Post").finish(),
                        Method::Put => f.debug_tuple("Method::Put").finish(),
                        Method::Delete => f.debug_tuple("Method::Delete").finish(),
                        Method::Connect => f.debug_tuple("Method::Connect").finish(),
                        Method::Options => f.debug_tuple("Method::Options").finish(),
                        Method::Trace => f.debug_tuple("Method::Trace").finish(),
                        Method::Patch => f.debug_tuple("Method::Patch").finish(),
                        Method::Other(e) => f.debug_tuple("Method::Other").field(e).finish(),
                    }
                }
            }
            /// This type corresponds to HTTP standard Related Schemes.
            pub enum Scheme {
                Http,
                Https,
                Other(plugin::wit_bindgen::rt::string::String),
            }
            #[automatically_derived]
            impl ::core::clone::Clone for Scheme {
                #[inline]
                fn clone(&self) -> Scheme {
                    match self {
                        Scheme::Http => Scheme::Http,
                        Scheme::Https => Scheme::Https,
                        Scheme::Other(__self_0) => {
                            Scheme::Other(::core::clone::Clone::clone(__self_0))
                        }
                    }
                }
            }
            impl ::core::fmt::Debug for Scheme {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    match self {
                        Scheme::Http => f.debug_tuple("Scheme::Http").finish(),
                        Scheme::Https => f.debug_tuple("Scheme::Https").finish(),
                        Scheme::Other(e) => f.debug_tuple("Scheme::Other").field(e).finish(),
                    }
                }
            }
            /// Defines the case payload type for `DNS-error` above:
            pub struct DnsErrorPayload {
                pub rcode: Option<plugin::wit_bindgen::rt::string::String>,
                pub info_code: Option<u16>,
            }
            #[automatically_derived]
            impl ::core::clone::Clone for DnsErrorPayload {
                #[inline]
                fn clone(&self) -> DnsErrorPayload {
                    DnsErrorPayload {
                        rcode: ::core::clone::Clone::clone(&self.rcode),
                        info_code: ::core::clone::Clone::clone(&self.info_code),
                    }
                }
            }
            impl ::core::fmt::Debug for DnsErrorPayload {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    f.debug_struct("DnsErrorPayload")
                        .field("rcode", &self.rcode)
                        .field("info-code", &self.info_code)
                        .finish()
                }
            }
            /// Defines the case payload type for `TLS-alert-received` above:
            pub struct TlsAlertReceivedPayload {
                pub alert_id: Option<u8>,
                pub alert_message: Option<plugin::wit_bindgen::rt::string::String>,
            }
            #[automatically_derived]
            impl ::core::clone::Clone for TlsAlertReceivedPayload {
                #[inline]
                fn clone(&self) -> TlsAlertReceivedPayload {
                    TlsAlertReceivedPayload {
                        alert_id: ::core::clone::Clone::clone(&self.alert_id),
                        alert_message: ::core::clone::Clone::clone(&self.alert_message),
                    }
                }
            }
            impl ::core::fmt::Debug for TlsAlertReceivedPayload {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    f.debug_struct("TlsAlertReceivedPayload")
                        .field("alert-id", &self.alert_id)
                        .field("alert-message", &self.alert_message)
                        .finish()
                }
            }
            /// Defines the case payload type for `HTTP-response-{header,trailer}-size` above:
            pub struct FieldSizePayload {
                pub field_name: Option<plugin::wit_bindgen::rt::string::String>,
                pub field_size: Option<u32>,
            }
            #[automatically_derived]
            impl ::core::clone::Clone for FieldSizePayload {
                #[inline]
                fn clone(&self) -> FieldSizePayload {
                    FieldSizePayload {
                        field_name: ::core::clone::Clone::clone(&self.field_name),
                        field_size: ::core::clone::Clone::clone(&self.field_size),
                    }
                }
            }
            impl ::core::fmt::Debug for FieldSizePayload {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    f.debug_struct("FieldSizePayload")
                        .field("field-name", &self.field_name)
                        .field("field-size", &self.field_size)
                        .finish()
                }
            }
            /// These cases are inspired by the IANA HTTP Proxy Error Types:
            /// https://www.iana.org/assignments/http-proxy-status/http-proxy-status.xhtml#table-http-proxy-error-types
            pub enum ErrorCode {
                DnsTimeout,
                DnsError(DnsErrorPayload),
                DestinationNotFound,
                DestinationUnavailable,
                DestinationIpProhibited,
                DestinationIpUnroutable,
                ConnectionRefused,
                ConnectionTerminated,
                ConnectionTimeout,
                ConnectionReadTimeout,
                ConnectionWriteTimeout,
                ConnectionLimitReached,
                TlsProtocolError,
                TlsCertificateError,
                TlsAlertReceived(TlsAlertReceivedPayload),
                HttpRequestDenied,
                HttpRequestLengthRequired,
                HttpRequestBodySize(Option<u64>),
                HttpRequestMethodInvalid,
                HttpRequestUriInvalid,
                HttpRequestUriTooLong,
                HttpRequestHeaderSectionSize(Option<u32>),
                HttpRequestHeaderSize(Option<FieldSizePayload>),
                HttpRequestTrailerSectionSize(Option<u32>),
                HttpRequestTrailerSize(FieldSizePayload),
                HttpResponseIncomplete,
                HttpResponseHeaderSectionSize(Option<u32>),
                HttpResponseHeaderSize(FieldSizePayload),
                HttpResponseBodySize(Option<u64>),
                HttpResponseTrailerSectionSize(Option<u32>),
                HttpResponseTrailerSize(FieldSizePayload),
                HttpResponseTransferCoding(Option<plugin::wit_bindgen::rt::string::String>),
                HttpResponseContentCoding(Option<plugin::wit_bindgen::rt::string::String>),
                HttpResponseTimeout,
                HttpUpgradeFailed,
                HttpProtocolError,
                LoopDetected,
                ConfigurationError,
                /// This is a catch-all error for anything that doesn't fit cleanly into a
                /// more specific case. It also includes an optional string for an
                /// unstructured description of the error. Users should not depend on the
                /// string for diagnosing errors, as it's not required to be consistent
                /// between implementations.
                InternalError(Option<plugin::wit_bindgen::rt::string::String>),
            }
            #[automatically_derived]
            impl ::core::clone::Clone for ErrorCode {
                #[inline]
                fn clone(&self) -> ErrorCode {
                    match self {
                        ErrorCode::DnsTimeout => ErrorCode::DnsTimeout,
                        ErrorCode::DnsError(__self_0) => {
                            ErrorCode::DnsError(::core::clone::Clone::clone(__self_0))
                        }
                        ErrorCode::DestinationNotFound => ErrorCode::DestinationNotFound,
                        ErrorCode::DestinationUnavailable => ErrorCode::DestinationUnavailable,
                        ErrorCode::DestinationIpProhibited => ErrorCode::DestinationIpProhibited,
                        ErrorCode::DestinationIpUnroutable => ErrorCode::DestinationIpUnroutable,
                        ErrorCode::ConnectionRefused => ErrorCode::ConnectionRefused,
                        ErrorCode::ConnectionTerminated => ErrorCode::ConnectionTerminated,
                        ErrorCode::ConnectionTimeout => ErrorCode::ConnectionTimeout,
                        ErrorCode::ConnectionReadTimeout => ErrorCode::ConnectionReadTimeout,
                        ErrorCode::ConnectionWriteTimeout => ErrorCode::ConnectionWriteTimeout,
                        ErrorCode::ConnectionLimitReached => ErrorCode::ConnectionLimitReached,
                        ErrorCode::TlsProtocolError => ErrorCode::TlsProtocolError,
                        ErrorCode::TlsCertificateError => ErrorCode::TlsCertificateError,
                        ErrorCode::TlsAlertReceived(__self_0) => {
                            ErrorCode::TlsAlertReceived(::core::clone::Clone::clone(__self_0))
                        }
                        ErrorCode::HttpRequestDenied => ErrorCode::HttpRequestDenied,
                        ErrorCode::HttpRequestLengthRequired => {
                            ErrorCode::HttpRequestLengthRequired
                        }
                        ErrorCode::HttpRequestBodySize(__self_0) => {
                            ErrorCode::HttpRequestBodySize(::core::clone::Clone::clone(__self_0))
                        }
                        ErrorCode::HttpRequestMethodInvalid => ErrorCode::HttpRequestMethodInvalid,
                        ErrorCode::HttpRequestUriInvalid => ErrorCode::HttpRequestUriInvalid,
                        ErrorCode::HttpRequestUriTooLong => ErrorCode::HttpRequestUriTooLong,
                        ErrorCode::HttpRequestHeaderSectionSize(__self_0) => {
                            ErrorCode::HttpRequestHeaderSectionSize(::core::clone::Clone::clone(
                                __self_0,
                            ))
                        }
                        ErrorCode::HttpRequestHeaderSize(__self_0) => {
                            ErrorCode::HttpRequestHeaderSize(::core::clone::Clone::clone(__self_0))
                        }
                        ErrorCode::HttpRequestTrailerSectionSize(__self_0) => {
                            ErrorCode::HttpRequestTrailerSectionSize(::core::clone::Clone::clone(
                                __self_0,
                            ))
                        }
                        ErrorCode::HttpRequestTrailerSize(__self_0) => {
                            ErrorCode::HttpRequestTrailerSize(::core::clone::Clone::clone(__self_0))
                        }
                        ErrorCode::HttpResponseIncomplete => ErrorCode::HttpResponseIncomplete,
                        ErrorCode::HttpResponseHeaderSectionSize(__self_0) => {
                            ErrorCode::HttpResponseHeaderSectionSize(::core::clone::Clone::clone(
                                __self_0,
                            ))
                        }
                        ErrorCode::HttpResponseHeaderSize(__self_0) => {
                            ErrorCode::HttpResponseHeaderSize(::core::clone::Clone::clone(__self_0))
                        }
                        ErrorCode::HttpResponseBodySize(__self_0) => {
                            ErrorCode::HttpResponseBodySize(::core::clone::Clone::clone(__self_0))
                        }
                        ErrorCode::HttpResponseTrailerSectionSize(__self_0) => {
                            ErrorCode::HttpResponseTrailerSectionSize(::core::clone::Clone::clone(
                                __self_0,
                            ))
                        }
                        ErrorCode::HttpResponseTrailerSize(__self_0) => {
                            ErrorCode::HttpResponseTrailerSize(::core::clone::Clone::clone(
                                __self_0,
                            ))
                        }
                        ErrorCode::HttpResponseTransferCoding(__self_0) => {
                            ErrorCode::HttpResponseTransferCoding(::core::clone::Clone::clone(
                                __self_0,
                            ))
                        }
                        ErrorCode::HttpResponseContentCoding(__self_0) => {
                            ErrorCode::HttpResponseContentCoding(::core::clone::Clone::clone(
                                __self_0,
                            ))
                        }
                        ErrorCode::HttpResponseTimeout => ErrorCode::HttpResponseTimeout,
                        ErrorCode::HttpUpgradeFailed => ErrorCode::HttpUpgradeFailed,
                        ErrorCode::HttpProtocolError => ErrorCode::HttpProtocolError,
                        ErrorCode::LoopDetected => ErrorCode::LoopDetected,
                        ErrorCode::ConfigurationError => ErrorCode::ConfigurationError,
                        ErrorCode::InternalError(__self_0) => {
                            ErrorCode::InternalError(::core::clone::Clone::clone(__self_0))
                        }
                    }
                }
            }
            impl ::core::fmt::Debug for ErrorCode {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    match self {
                        ErrorCode::DnsTimeout => f.debug_tuple("ErrorCode::DnsTimeout").finish(),
                        ErrorCode::DnsError(e) => {
                            f.debug_tuple("ErrorCode::DnsError").field(e).finish()
                        }
                        ErrorCode::DestinationNotFound => {
                            f.debug_tuple("ErrorCode::DestinationNotFound").finish()
                        }
                        ErrorCode::DestinationUnavailable => {
                            f.debug_tuple("ErrorCode::DestinationUnavailable").finish()
                        }
                        ErrorCode::DestinationIpProhibited => {
                            f.debug_tuple("ErrorCode::DestinationIpProhibited").finish()
                        }
                        ErrorCode::DestinationIpUnroutable => {
                            f.debug_tuple("ErrorCode::DestinationIpUnroutable").finish()
                        }
                        ErrorCode::ConnectionRefused => {
                            f.debug_tuple("ErrorCode::ConnectionRefused").finish()
                        }
                        ErrorCode::ConnectionTerminated => {
                            f.debug_tuple("ErrorCode::ConnectionTerminated").finish()
                        }
                        ErrorCode::ConnectionTimeout => {
                            f.debug_tuple("ErrorCode::ConnectionTimeout").finish()
                        }
                        ErrorCode::ConnectionReadTimeout => {
                            f.debug_tuple("ErrorCode::ConnectionReadTimeout").finish()
                        }
                        ErrorCode::ConnectionWriteTimeout => {
                            f.debug_tuple("ErrorCode::ConnectionWriteTimeout").finish()
                        }
                        ErrorCode::ConnectionLimitReached => {
                            f.debug_tuple("ErrorCode::ConnectionLimitReached").finish()
                        }
                        ErrorCode::TlsProtocolError => {
                            f.debug_tuple("ErrorCode::TlsProtocolError").finish()
                        }
                        ErrorCode::TlsCertificateError => {
                            f.debug_tuple("ErrorCode::TlsCertificateError").finish()
                        }
                        ErrorCode::TlsAlertReceived(e) => f
                            .debug_tuple("ErrorCode::TlsAlertReceived")
                            .field(e)
                            .finish(),
                        ErrorCode::HttpRequestDenied => {
                            f.debug_tuple("ErrorCode::HttpRequestDenied").finish()
                        }
                        ErrorCode::HttpRequestLengthRequired => f
                            .debug_tuple("ErrorCode::HttpRequestLengthRequired")
                            .finish(),
                        ErrorCode::HttpRequestBodySize(e) => f
                            .debug_tuple("ErrorCode::HttpRequestBodySize")
                            .field(e)
                            .finish(),
                        ErrorCode::HttpRequestMethodInvalid => f
                            .debug_tuple("ErrorCode::HttpRequestMethodInvalid")
                            .finish(),
                        ErrorCode::HttpRequestUriInvalid => {
                            f.debug_tuple("ErrorCode::HttpRequestUriInvalid").finish()
                        }
                        ErrorCode::HttpRequestUriTooLong => {
                            f.debug_tuple("ErrorCode::HttpRequestUriTooLong").finish()
                        }
                        ErrorCode::HttpRequestHeaderSectionSize(e) => f
                            .debug_tuple("ErrorCode::HttpRequestHeaderSectionSize")
                            .field(e)
                            .finish(),
                        ErrorCode::HttpRequestHeaderSize(e) => f
                            .debug_tuple("ErrorCode::HttpRequestHeaderSize")
                            .field(e)
                            .finish(),
                        ErrorCode::HttpRequestTrailerSectionSize(e) => f
                            .debug_tuple("ErrorCode::HttpRequestTrailerSectionSize")
                            .field(e)
                            .finish(),
                        ErrorCode::HttpRequestTrailerSize(e) => f
                            .debug_tuple("ErrorCode::HttpRequestTrailerSize")
                            .field(e)
                            .finish(),
                        ErrorCode::HttpResponseIncomplete => {
                            f.debug_tuple("ErrorCode::HttpResponseIncomplete").finish()
                        }
                        ErrorCode::HttpResponseHeaderSectionSize(e) => f
                            .debug_tuple("ErrorCode::HttpResponseHeaderSectionSize")
                            .field(e)
                            .finish(),
                        ErrorCode::HttpResponseHeaderSize(e) => f
                            .debug_tuple("ErrorCode::HttpResponseHeaderSize")
                            .field(e)
                            .finish(),
                        ErrorCode::HttpResponseBodySize(e) => f
                            .debug_tuple("ErrorCode::HttpResponseBodySize")
                            .field(e)
                            .finish(),
                        ErrorCode::HttpResponseTrailerSectionSize(e) => f
                            .debug_tuple("ErrorCode::HttpResponseTrailerSectionSize")
                            .field(e)
                            .finish(),
                        ErrorCode::HttpResponseTrailerSize(e) => f
                            .debug_tuple("ErrorCode::HttpResponseTrailerSize")
                            .field(e)
                            .finish(),
                        ErrorCode::HttpResponseTransferCoding(e) => f
                            .debug_tuple("ErrorCode::HttpResponseTransferCoding")
                            .field(e)
                            .finish(),
                        ErrorCode::HttpResponseContentCoding(e) => f
                            .debug_tuple("ErrorCode::HttpResponseContentCoding")
                            .field(e)
                            .finish(),
                        ErrorCode::HttpResponseTimeout => {
                            f.debug_tuple("ErrorCode::HttpResponseTimeout").finish()
                        }
                        ErrorCode::HttpUpgradeFailed => {
                            f.debug_tuple("ErrorCode::HttpUpgradeFailed").finish()
                        }
                        ErrorCode::HttpProtocolError => {
                            f.debug_tuple("ErrorCode::HttpProtocolError").finish()
                        }
                        ErrorCode::LoopDetected => {
                            f.debug_tuple("ErrorCode::LoopDetected").finish()
                        }
                        ErrorCode::ConfigurationError => {
                            f.debug_tuple("ErrorCode::ConfigurationError").finish()
                        }
                        ErrorCode::InternalError(e) => {
                            f.debug_tuple("ErrorCode::InternalError").field(e).finish()
                        }
                    }
                }
            }
            impl ::core::fmt::Display for ErrorCode {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    f.write_fmt(format_args!("{0:?}", self))
                }
            }
            impl std::error::Error for ErrorCode {}
            /// This type enumerates the different kinds of errors that may occur when
            /// setting or appending to a `fields` resource.
            pub enum HeaderError {
                /// This error indicates that a `field-key` or `field-value` was
                /// syntactically invalid when used with an operation that sets headers in a
                /// `fields`.
                InvalidSyntax,
                /// This error indicates that a forbidden `field-key` was used when trying
                /// to set a header in a `fields`.
                Forbidden,
                /// This error indicates that the operation on the `fields` was not
                /// permitted because the fields are immutable.
                Immutable,
            }
            #[automatically_derived]
            impl ::core::clone::Clone for HeaderError {
                #[inline]
                fn clone(&self) -> HeaderError {
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for HeaderError {}
            impl ::core::fmt::Debug for HeaderError {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    match self {
                        HeaderError::InvalidSyntax => {
                            f.debug_tuple("HeaderError::InvalidSyntax").finish()
                        }
                        HeaderError::Forbidden => f.debug_tuple("HeaderError::Forbidden").finish(),
                        HeaderError::Immutable => f.debug_tuple("HeaderError::Immutable").finish(),
                    }
                }
            }
            impl ::core::fmt::Display for HeaderError {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    f.write_fmt(format_args!("{0:?}", self))
                }
            }
            impl std::error::Error for HeaderError {}
            /// Field keys are always strings.
            pub type FieldKey = plugin::wit_bindgen::rt::string::String;
            /// Field values should always be ASCII strings. However, in
            /// reality, HTTP implementations often have to interpret malformed values,
            /// so they are provided as a list of bytes.
            pub type FieldValue = plugin::wit_bindgen::rt::vec::Vec<u8>;
            /// This following block defines the `fields` resource which corresponds to
            /// HTTP standard Fields. Fields are a common representation used for both
            /// Headers and Trailers.
            ///
            /// A `fields` may be mutable or immutable. A `fields` created using the
            /// constructor, `from-list`, or `clone` will be mutable, but a `fields`
            /// resource given by other means (including, but not limited to,
            /// `incoming-request.headers`, `outgoing-request.headers`) might be be
            /// immutable. In an immutable fields, the `set`, `append`, and `delete`
            /// operations will fail with `header-error.immutable`.
            #[repr(transparent)]
            pub struct Fields {
                handle: plugin::wit_bindgen::rt::Resource<Fields>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for Fields {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "Fields",
                        "handle",
                        &&self.handle,
                    )
                }
            }
            impl Fields {
                #[doc(hidden)]
                pub unsafe fn from_handle(handle: u32) -> Self {
                    Self {
                        handle: plugin::wit_bindgen::rt::Resource::from_handle(handle),
                    }
                }
                #[doc(hidden)]
                pub fn into_handle(self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::into_handle(self.handle)
                }
                #[doc(hidden)]
                pub fn handle(&self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::handle(&self.handle)
                }
            }
            unsafe impl plugin::wit_bindgen::rt::WasmResource for Fields {
                #[inline]
                unsafe fn drop(_handle: u32) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[resource-drop]fields"]
                            fn drop(_: u32);
                        }
                        drop(_handle);
                    }
                }
            }
            /// Headers is an alias for Fields.
            pub type Headers = Fields;
            /// Trailers is an alias for Fields.
            pub type Trailers = Fields;
            /// Represents an incoming HTTP Request.
            #[repr(transparent)]
            pub struct IncomingRequest {
                handle: plugin::wit_bindgen::rt::Resource<IncomingRequest>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for IncomingRequest {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "IncomingRequest",
                        "handle",
                        &&self.handle,
                    )
                }
            }
            impl IncomingRequest {
                #[doc(hidden)]
                pub unsafe fn from_handle(handle: u32) -> Self {
                    Self {
                        handle: plugin::wit_bindgen::rt::Resource::from_handle(handle),
                    }
                }
                #[doc(hidden)]
                pub fn into_handle(self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::into_handle(self.handle)
                }
                #[doc(hidden)]
                pub fn handle(&self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::handle(&self.handle)
                }
            }
            unsafe impl plugin::wit_bindgen::rt::WasmResource for IncomingRequest {
                #[inline]
                unsafe fn drop(_handle: u32) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[resource-drop]incoming-request"]
                            fn drop(_: u32);
                        }
                        drop(_handle);
                    }
                }
            }
            /// Represents an outgoing HTTP Request.
            #[repr(transparent)]
            pub struct OutgoingRequest {
                handle: plugin::wit_bindgen::rt::Resource<OutgoingRequest>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for OutgoingRequest {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "OutgoingRequest",
                        "handle",
                        &&self.handle,
                    )
                }
            }
            impl OutgoingRequest {
                #[doc(hidden)]
                pub unsafe fn from_handle(handle: u32) -> Self {
                    Self {
                        handle: plugin::wit_bindgen::rt::Resource::from_handle(handle),
                    }
                }
                #[doc(hidden)]
                pub fn into_handle(self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::into_handle(self.handle)
                }
                #[doc(hidden)]
                pub fn handle(&self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::handle(&self.handle)
                }
            }
            unsafe impl plugin::wit_bindgen::rt::WasmResource for OutgoingRequest {
                #[inline]
                unsafe fn drop(_handle: u32) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[resource-drop]outgoing-request"]
                            fn drop(_: u32);
                        }
                        drop(_handle);
                    }
                }
            }
            /// Parameters for making an HTTP Request. Each of these parameters is
            /// currently an optional timeout applicable to the transport layer of the
            /// HTTP protocol.
            ///
            /// These timeouts are separate from any the user may use to bound a
            /// blocking call to `wasi:io/poll.poll`.
            #[repr(transparent)]
            pub struct RequestOptions {
                handle: plugin::wit_bindgen::rt::Resource<RequestOptions>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for RequestOptions {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "RequestOptions",
                        "handle",
                        &&self.handle,
                    )
                }
            }
            impl RequestOptions {
                #[doc(hidden)]
                pub unsafe fn from_handle(handle: u32) -> Self {
                    Self {
                        handle: plugin::wit_bindgen::rt::Resource::from_handle(handle),
                    }
                }
                #[doc(hidden)]
                pub fn into_handle(self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::into_handle(self.handle)
                }
                #[doc(hidden)]
                pub fn handle(&self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::handle(&self.handle)
                }
            }
            unsafe impl plugin::wit_bindgen::rt::WasmResource for RequestOptions {
                #[inline]
                unsafe fn drop(_handle: u32) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[resource-drop]request-options"]
                            fn drop(_: u32);
                        }
                        drop(_handle);
                    }
                }
            }
            /// Represents the ability to send an HTTP Response.
            ///
            /// This resource is used by the `wasi:http/incoming-handler` interface to
            /// allow a Response to be sent corresponding to the Request provided as the
            /// other argument to `incoming-handler.handle`.
            #[repr(transparent)]
            pub struct ResponseOutparam {
                handle: plugin::wit_bindgen::rt::Resource<ResponseOutparam>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for ResponseOutparam {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "ResponseOutparam",
                        "handle",
                        &&self.handle,
                    )
                }
            }
            impl ResponseOutparam {
                #[doc(hidden)]
                pub unsafe fn from_handle(handle: u32) -> Self {
                    Self {
                        handle: plugin::wit_bindgen::rt::Resource::from_handle(handle),
                    }
                }
                #[doc(hidden)]
                pub fn into_handle(self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::into_handle(self.handle)
                }
                #[doc(hidden)]
                pub fn handle(&self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::handle(&self.handle)
                }
            }
            unsafe impl plugin::wit_bindgen::rt::WasmResource for ResponseOutparam {
                #[inline]
                unsafe fn drop(_handle: u32) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[resource-drop]response-outparam"]
                            fn drop(_: u32);
                        }
                        drop(_handle);
                    }
                }
            }
            /// This type corresponds to the HTTP standard Status Code.
            pub type StatusCode = u16;
            /// Represents an incoming HTTP Response.
            #[repr(transparent)]
            pub struct IncomingResponse {
                handle: plugin::wit_bindgen::rt::Resource<IncomingResponse>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for IncomingResponse {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "IncomingResponse",
                        "handle",
                        &&self.handle,
                    )
                }
            }
            impl IncomingResponse {
                #[doc(hidden)]
                pub unsafe fn from_handle(handle: u32) -> Self {
                    Self {
                        handle: plugin::wit_bindgen::rt::Resource::from_handle(handle),
                    }
                }
                #[doc(hidden)]
                pub fn into_handle(self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::into_handle(self.handle)
                }
                #[doc(hidden)]
                pub fn handle(&self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::handle(&self.handle)
                }
            }
            unsafe impl plugin::wit_bindgen::rt::WasmResource for IncomingResponse {
                #[inline]
                unsafe fn drop(_handle: u32) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[resource-drop]incoming-response"]
                            fn drop(_: u32);
                        }
                        drop(_handle);
                    }
                }
            }
            /// Represents an incoming HTTP Request or Response's Body.
            ///
            /// A body has both its contents - a stream of bytes - and a (possibly
            /// empty) set of trailers, indicating that the full contents of the
            /// body have been received. This resource represents the contents as
            /// an `input-stream` and the delivery of trailers as a `future-trailers`,
            /// and ensures that the user of this interface may only be consuming either
            /// the body contents or waiting on trailers at any given time.
            #[repr(transparent)]
            pub struct IncomingBody {
                handle: plugin::wit_bindgen::rt::Resource<IncomingBody>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for IncomingBody {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "IncomingBody",
                        "handle",
                        &&self.handle,
                    )
                }
            }
            impl IncomingBody {
                #[doc(hidden)]
                pub unsafe fn from_handle(handle: u32) -> Self {
                    Self {
                        handle: plugin::wit_bindgen::rt::Resource::from_handle(handle),
                    }
                }
                #[doc(hidden)]
                pub fn into_handle(self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::into_handle(self.handle)
                }
                #[doc(hidden)]
                pub fn handle(&self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::handle(&self.handle)
                }
            }
            unsafe impl plugin::wit_bindgen::rt::WasmResource for IncomingBody {
                #[inline]
                unsafe fn drop(_handle: u32) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[resource-drop]incoming-body"]
                            fn drop(_: u32);
                        }
                        drop(_handle);
                    }
                }
            }
            /// Represents a future which may eventaully return trailers, or an error.
            ///
            /// In the case that the incoming HTTP Request or Response did not have any
            /// trailers, this future will resolve to the empty set of trailers once the
            /// complete Request or Response body has been received.
            #[repr(transparent)]
            pub struct FutureTrailers {
                handle: plugin::wit_bindgen::rt::Resource<FutureTrailers>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for FutureTrailers {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "FutureTrailers",
                        "handle",
                        &&self.handle,
                    )
                }
            }
            impl FutureTrailers {
                #[doc(hidden)]
                pub unsafe fn from_handle(handle: u32) -> Self {
                    Self {
                        handle: plugin::wit_bindgen::rt::Resource::from_handle(handle),
                    }
                }
                #[doc(hidden)]
                pub fn into_handle(self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::into_handle(self.handle)
                }
                #[doc(hidden)]
                pub fn handle(&self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::handle(&self.handle)
                }
            }
            unsafe impl plugin::wit_bindgen::rt::WasmResource for FutureTrailers {
                #[inline]
                unsafe fn drop(_handle: u32) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[resource-drop]future-trailers"]
                            fn drop(_: u32);
                        }
                        drop(_handle);
                    }
                }
            }
            /// Represents an outgoing HTTP Response.
            #[repr(transparent)]
            pub struct OutgoingResponse {
                handle: plugin::wit_bindgen::rt::Resource<OutgoingResponse>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for OutgoingResponse {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "OutgoingResponse",
                        "handle",
                        &&self.handle,
                    )
                }
            }
            impl OutgoingResponse {
                #[doc(hidden)]
                pub unsafe fn from_handle(handle: u32) -> Self {
                    Self {
                        handle: plugin::wit_bindgen::rt::Resource::from_handle(handle),
                    }
                }
                #[doc(hidden)]
                pub fn into_handle(self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::into_handle(self.handle)
                }
                #[doc(hidden)]
                pub fn handle(&self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::handle(&self.handle)
                }
            }
            unsafe impl plugin::wit_bindgen::rt::WasmResource for OutgoingResponse {
                #[inline]
                unsafe fn drop(_handle: u32) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[resource-drop]outgoing-response"]
                            fn drop(_: u32);
                        }
                        drop(_handle);
                    }
                }
            }
            /// Represents an outgoing HTTP Request or Response's Body.
            ///
            /// A body has both its contents - a stream of bytes - and a (possibly
            /// empty) set of trailers, inducating the full contents of the body
            /// have been sent. This resource represents the contents as an
            /// `output-stream` child resource, and the completion of the body (with
            /// optional trailers) with a static function that consumes the
            /// `outgoing-body` resource, and ensures that the user of this interface
            /// may not write to the body contents after the body has been finished.
            ///
            /// If the user code drops this resource, as opposed to calling the static
            /// method `finish`, the implementation should treat the body as incomplete,
            /// and that an error has occured. The implementation should propogate this
            /// error to the HTTP protocol by whatever means it has available,
            /// including: corrupting the body on the wire, aborting the associated
            /// Request, or sending a late status code for the Response.
            #[repr(transparent)]
            pub struct OutgoingBody {
                handle: plugin::wit_bindgen::rt::Resource<OutgoingBody>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for OutgoingBody {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "OutgoingBody",
                        "handle",
                        &&self.handle,
                    )
                }
            }
            impl OutgoingBody {
                #[doc(hidden)]
                pub unsafe fn from_handle(handle: u32) -> Self {
                    Self {
                        handle: plugin::wit_bindgen::rt::Resource::from_handle(handle),
                    }
                }
                #[doc(hidden)]
                pub fn into_handle(self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::into_handle(self.handle)
                }
                #[doc(hidden)]
                pub fn handle(&self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::handle(&self.handle)
                }
            }
            unsafe impl plugin::wit_bindgen::rt::WasmResource for OutgoingBody {
                #[inline]
                unsafe fn drop(_handle: u32) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[resource-drop]outgoing-body"]
                            fn drop(_: u32);
                        }
                        drop(_handle);
                    }
                }
            }
            /// Represents a future which may eventaully return an incoming HTTP
            /// Response, or an error.
            ///
            /// This resource is returned by the `wasi:http/outgoing-handler` interface to
            /// provide the HTTP Response corresponding to the sent Request.
            #[repr(transparent)]
            pub struct FutureIncomingResponse {
                handle: plugin::wit_bindgen::rt::Resource<FutureIncomingResponse>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for FutureIncomingResponse {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "FutureIncomingResponse",
                        "handle",
                        &&self.handle,
                    )
                }
            }
            impl FutureIncomingResponse {
                #[doc(hidden)]
                pub unsafe fn from_handle(handle: u32) -> Self {
                    Self {
                        handle: plugin::wit_bindgen::rt::Resource::from_handle(handle),
                    }
                }
                #[doc(hidden)]
                pub fn into_handle(self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::into_handle(self.handle)
                }
                #[doc(hidden)]
                pub fn handle(&self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::handle(&self.handle)
                }
            }
            unsafe impl plugin::wit_bindgen::rt::WasmResource for FutureIncomingResponse {
                #[inline]
                unsafe fn drop(_handle: u32) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[resource-drop]future-incoming-response"]
                            fn drop(_: u32);
                        }
                        drop(_handle);
                    }
                }
            }
            #[allow(unused_unsafe, clippy::all)]
            /// Attempts to extract a http-related `error` from the wasi:io `error`
            /// provided.
            ///
            /// Stream operations which return
            /// `wasi:io/stream/stream-error::last-operation-failed` have a payload of
            /// type `wasi:io/error/error` with more information about the operation
            /// that failed. This payload can be passed through to this function to see
            /// if there's http-related information about the error to return.
            ///
            /// Note that this function is fallible because not all io-errors are
            /// http-related errors.
            pub fn http_error_code(err: &IoError) -> Option<ErrorCode> {
                #[allow(unused_imports)]
                use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                unsafe {
                    #[repr(align(8))]
                    struct RetArea([u8; 40]);
                    let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                    let ptr0 = ret_area.as_mut_ptr() as i32;
                    #[cfg(target_arch = "wasm32")]
                    #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                    extern "C" {
                        #[link_name = "http-error-code"]
                        fn wit_import(_: i32, _: i32);
                    }
                    wit_import((err).handle() as i32, ptr0);
                    let l1 = i32::from(*((ptr0 + 0) as *const u8));
                    match l1 {
                        0 => None,
                        1 => {
                            let e = {
                                let l2 = i32::from(*((ptr0 + 8) as *const u8));
                                let v64 = match l2 {
                                    0 => ErrorCode::DnsTimeout,
                                    1 => {
                                        let e64 = {
                                            let l3 = i32::from(*((ptr0 + 16) as *const u8));
                                            let l7 = i32::from(*((ptr0 + 28) as *const u8));
                                            DnsErrorPayload { rcode : match l3 { 0 => None , 1 => { let e = { let l4 = * ((ptr0 + 20) as * const i32) ; let l5 = * ((ptr0 + 24) as * const i32) ; let len6 = l5 as usize ; let bytes6 = Vec :: from_raw_parts (l4 as * mut _ , len6 , len6) ; plugin :: wit_bindgen :: rt :: string_lift (bytes6) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , info_code : match l7 { 0 => None , 1 => { let e = { let l8 = i32 :: from (* ((ptr0 + 30) as * const u16)) ; l8 as u16 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , }
                                        };
                                        ErrorCode::DnsError(e64)
                                    }
                                    2 => ErrorCode::DestinationNotFound,
                                    3 => ErrorCode::DestinationUnavailable,
                                    4 => ErrorCode::DestinationIpProhibited,
                                    5 => ErrorCode::DestinationIpUnroutable,
                                    6 => ErrorCode::ConnectionRefused,
                                    7 => ErrorCode::ConnectionTerminated,
                                    8 => ErrorCode::ConnectionTimeout,
                                    9 => ErrorCode::ConnectionReadTimeout,
                                    10 => ErrorCode::ConnectionWriteTimeout,
                                    11 => ErrorCode::ConnectionLimitReached,
                                    12 => ErrorCode::TlsProtocolError,
                                    13 => ErrorCode::TlsCertificateError,
                                    14 => {
                                        let e64 = {
                                            let l9 = i32::from(*((ptr0 + 16) as *const u8));
                                            let l11 = i32::from(*((ptr0 + 20) as *const u8));
                                            TlsAlertReceivedPayload { alert_id : match l9 { 0 => None , 1 => { let e = { let l10 = i32 :: from (* ((ptr0 + 17) as * const u8)) ; l10 as u8 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , alert_message : match l11 { 0 => None , 1 => { let e = { let l12 = * ((ptr0 + 24) as * const i32) ; let l13 = * ((ptr0 + 28) as * const i32) ; let len14 = l13 as usize ; let bytes14 = Vec :: from_raw_parts (l12 as * mut _ , len14 , len14) ; plugin :: wit_bindgen :: rt :: string_lift (bytes14) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , }
                                        };
                                        ErrorCode::TlsAlertReceived(e64)
                                    }
                                    15 => ErrorCode::HttpRequestDenied,
                                    16 => ErrorCode::HttpRequestLengthRequired,
                                    17 => {
                                        let e64 = {
                                            let l15 = i32::from(*((ptr0 + 16) as *const u8));
                                            match l15 { 0 => None , 1 => { let e = { let l16 = * ((ptr0 + 24) as * const i64) ; l16 as u64 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        ErrorCode::HttpRequestBodySize(e64)
                                    }
                                    18 => ErrorCode::HttpRequestMethodInvalid,
                                    19 => ErrorCode::HttpRequestUriInvalid,
                                    20 => ErrorCode::HttpRequestUriTooLong,
                                    21 => {
                                        let e64 = {
                                            let l17 = i32::from(*((ptr0 + 16) as *const u8));
                                            match l17 { 0 => None , 1 => { let e = { let l18 = * ((ptr0 + 20) as * const i32) ; l18 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        ErrorCode::HttpRequestHeaderSectionSize(e64)
                                    }
                                    22 => {
                                        let e64 = {
                                            let l19 = i32::from(*((ptr0 + 16) as *const u8));
                                            match l19 { 0 => None , 1 => { let e = { let l20 = i32 :: from (* ((ptr0 + 20) as * const u8)) ; let l24 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; FieldSizePayload { field_name : match l20 { 0 => None , 1 => { let e = { let l21 = * ((ptr0 + 24) as * const i32) ; let l22 = * ((ptr0 + 28) as * const i32) ; let len23 = l22 as usize ; let bytes23 = Vec :: from_raw_parts (l21 as * mut _ , len23 , len23) ; plugin :: wit_bindgen :: rt :: string_lift (bytes23) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l24 { 0 => None , 1 => { let e = { let l25 = * ((ptr0 + 36) as * const i32) ; l25 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , } } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        ErrorCode::HttpRequestHeaderSize(e64)
                                    }
                                    23 => {
                                        let e64 = {
                                            let l26 = i32::from(*((ptr0 + 16) as *const u8));
                                            match l26 { 0 => None , 1 => { let e = { let l27 = * ((ptr0 + 20) as * const i32) ; l27 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        ErrorCode::HttpRequestTrailerSectionSize(e64)
                                    }
                                    24 => {
                                        let e64 = {
                                            let l28 = i32::from(*((ptr0 + 16) as *const u8));
                                            let l32 = i32::from(*((ptr0 + 28) as *const u8));
                                            FieldSizePayload { field_name : match l28 { 0 => None , 1 => { let e = { let l29 = * ((ptr0 + 20) as * const i32) ; let l30 = * ((ptr0 + 24) as * const i32) ; let len31 = l30 as usize ; let bytes31 = Vec :: from_raw_parts (l29 as * mut _ , len31 , len31) ; plugin :: wit_bindgen :: rt :: string_lift (bytes31) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l32 { 0 => None , 1 => { let e = { let l33 = * ((ptr0 + 32) as * const i32) ; l33 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , }
                                        };
                                        ErrorCode::HttpRequestTrailerSize(e64)
                                    }
                                    25 => ErrorCode::HttpResponseIncomplete,
                                    26 => {
                                        let e64 = {
                                            let l34 = i32::from(*((ptr0 + 16) as *const u8));
                                            match l34 { 0 => None , 1 => { let e = { let l35 = * ((ptr0 + 20) as * const i32) ; l35 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        ErrorCode::HttpResponseHeaderSectionSize(e64)
                                    }
                                    27 => {
                                        let e64 = {
                                            let l36 = i32::from(*((ptr0 + 16) as *const u8));
                                            let l40 = i32::from(*((ptr0 + 28) as *const u8));
                                            FieldSizePayload { field_name : match l36 { 0 => None , 1 => { let e = { let l37 = * ((ptr0 + 20) as * const i32) ; let l38 = * ((ptr0 + 24) as * const i32) ; let len39 = l38 as usize ; let bytes39 = Vec :: from_raw_parts (l37 as * mut _ , len39 , len39) ; plugin :: wit_bindgen :: rt :: string_lift (bytes39) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l40 { 0 => None , 1 => { let e = { let l41 = * ((ptr0 + 32) as * const i32) ; l41 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , }
                                        };
                                        ErrorCode::HttpResponseHeaderSize(e64)
                                    }
                                    28 => {
                                        let e64 = {
                                            let l42 = i32::from(*((ptr0 + 16) as *const u8));
                                            match l42 { 0 => None , 1 => { let e = { let l43 = * ((ptr0 + 24) as * const i64) ; l43 as u64 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        ErrorCode::HttpResponseBodySize(e64)
                                    }
                                    29 => {
                                        let e64 = {
                                            let l44 = i32::from(*((ptr0 + 16) as *const u8));
                                            match l44 { 0 => None , 1 => { let e = { let l45 = * ((ptr0 + 20) as * const i32) ; l45 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        ErrorCode::HttpResponseTrailerSectionSize(e64)
                                    }
                                    30 => {
                                        let e64 = {
                                            let l46 = i32::from(*((ptr0 + 16) as *const u8));
                                            let l50 = i32::from(*((ptr0 + 28) as *const u8));
                                            FieldSizePayload { field_name : match l46 { 0 => None , 1 => { let e = { let l47 = * ((ptr0 + 20) as * const i32) ; let l48 = * ((ptr0 + 24) as * const i32) ; let len49 = l48 as usize ; let bytes49 = Vec :: from_raw_parts (l47 as * mut _ , len49 , len49) ; plugin :: wit_bindgen :: rt :: string_lift (bytes49) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l50 { 0 => None , 1 => { let e = { let l51 = * ((ptr0 + 32) as * const i32) ; l51 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , }
                                        };
                                        ErrorCode::HttpResponseTrailerSize(e64)
                                    }
                                    31 => {
                                        let e64 = {
                                            let l52 = i32::from(*((ptr0 + 16) as *const u8));
                                            match l52 { 0 => None , 1 => { let e = { let l53 = * ((ptr0 + 20) as * const i32) ; let l54 = * ((ptr0 + 24) as * const i32) ; let len55 = l54 as usize ; let bytes55 = Vec :: from_raw_parts (l53 as * mut _ , len55 , len55) ; plugin :: wit_bindgen :: rt :: string_lift (bytes55) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        ErrorCode::HttpResponseTransferCoding(e64)
                                    }
                                    32 => {
                                        let e64 = {
                                            let l56 = i32::from(*((ptr0 + 16) as *const u8));
                                            match l56 { 0 => None , 1 => { let e = { let l57 = * ((ptr0 + 20) as * const i32) ; let l58 = * ((ptr0 + 24) as * const i32) ; let len59 = l58 as usize ; let bytes59 = Vec :: from_raw_parts (l57 as * mut _ , len59 , len59) ; plugin :: wit_bindgen :: rt :: string_lift (bytes59) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        ErrorCode::HttpResponseContentCoding(e64)
                                    }
                                    33 => ErrorCode::HttpResponseTimeout,
                                    34 => ErrorCode::HttpUpgradeFailed,
                                    35 => ErrorCode::HttpProtocolError,
                                    36 => ErrorCode::LoopDetected,
                                    37 => ErrorCode::ConfigurationError,
                                    n => {
                                        if true {
                                            match (&n, &38) {
                                                (left_val, right_val) => {
                                                    if !(*left_val == *right_val) {
                                                        let kind =
                                                            ::core::panicking::AssertKind::Eq;
                                                        ::core::panicking::assert_failed(
                                                            kind,
                                                            &*left_val,
                                                            &*right_val,
                                                            ::core::option::Option::Some(
                                                                format_args!(
                                                                    "invalid enum discriminant"
                                                                ),
                                                            ),
                                                        );
                                                    }
                                                }
                                            };
                                        };
                                        let e64 = {
                                            let l60 = i32::from(*((ptr0 + 16) as *const u8));
                                            match l60 { 0 => None , 1 => { let e = { let l61 = * ((ptr0 + 20) as * const i32) ; let l62 = * ((ptr0 + 24) as * const i32) ; let len63 = l62 as usize ; let bytes63 = Vec :: from_raw_parts (l61 as * mut _ , len63 , len63) ; plugin :: wit_bindgen :: rt :: string_lift (bytes63) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        ErrorCode::InternalError(e64)
                                    }
                                };
                                v64
                            };
                            Some(e)
                        }
                        _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                    }
                }
            }
            impl Fields {
                #[allow(unused_unsafe, clippy::all)]
                /// Construct an empty HTTP Fields.
                ///
                /// The resulting `fields` is mutable.
                pub fn new() -> Self {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[constructor]fields"]
                            fn wit_import() -> i32;
                        }
                        let ret = wit_import();
                        Fields::from_handle(ret as u32)
                    }
                }
            }
            impl Fields {
                #[allow(unused_unsafe, clippy::all)]
                /// Construct an HTTP Fields.
                ///
                /// The resulting `fields` is mutable.
                ///
                /// The list represents each key-value pair in the Fields. Keys
                /// which have multiple values are represented by multiple entries in this
                /// list with the same key.
                ///
                /// The tuple is a pair of the field key, represented as a string, and
                /// Value, represented as a list of bytes. In a valid Fields, all keys
                /// and values are valid UTF-8 strings. However, values are not always
                /// well-formed, so they are represented as a raw list of bytes.
                ///
                /// An error result will be returned if any header or value was
                /// syntactically invalid, or if a header was forbidden.
                pub fn from_list(
                    entries: &[(FieldKey, FieldValue)],
                ) -> Result<Fields, HeaderError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 8]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let vec3 = entries;
                        let len3 = vec3.len() as i32;
                        let layout3 = alloc::Layout::from_size_align_unchecked(vec3.len() * 16, 4);
                        let result3 = if layout3.size() != 0 {
                            let ptr = alloc::alloc(layout3);
                            if ptr.is_null() {
                                alloc::handle_alloc_error(layout3);
                            }
                            ptr
                        } else {
                            {
                                ::core::ptr::null_mut()
                            }
                        };
                        for (i, e) in vec3.into_iter().enumerate() {
                            let base = result3 as i32 + (i as i32) * 16;
                            {
                                let (t0_0, t0_1) = e;
                                let vec1 = t0_0;
                                let ptr1 = vec1.as_ptr() as i32;
                                let len1 = vec1.len() as i32;
                                *((base + 4) as *mut i32) = len1;
                                *((base + 0) as *mut i32) = ptr1;
                                let vec2 = t0_1;
                                let ptr2 = vec2.as_ptr() as i32;
                                let len2 = vec2.len() as i32;
                                *((base + 12) as *mut i32) = len2;
                                *((base + 8) as *mut i32) = ptr2;
                            }
                        }
                        let ptr4 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[static]fields.from-list"]
                            fn wit_import(_: i32, _: i32, _: i32);
                        }
                        wit_import(result3 as i32, len3, ptr4);
                        let l5 = i32::from(*((ptr4 + 0) as *const u8));
                        if layout3.size() != 0 {
                            alloc::dealloc(result3, layout3);
                        }
                        match l5 {
                            0 => {
                                let e = {
                                    let l6 = *((ptr4 + 4) as *const i32);
                                    Fields::from_handle(l6 as u32)
                                };
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l7 = i32::from(*((ptr4 + 4) as *const u8));
                                    let v8 = match l7 {
                                        0 => HeaderError::InvalidSyntax,
                                        1 => HeaderError::Forbidden,
                                        n => {
                                            if true {
                                                match (&n, &2) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            HeaderError::Immutable
                                        }
                                    };
                                    v8
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl Fields {
                #[allow(unused_unsafe, clippy::all)]
                /// Get all of the values corresponding to a key. If the key is not present
                /// in this `fields`, an empty list is returned. However, if the key is
                /// present but empty, this is represented by a list with one or more
                /// empty field-values present.
                pub fn get(
                    &self,
                    name: &FieldKey,
                ) -> plugin::wit_bindgen::rt::vec::Vec<FieldValue> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 8]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let vec0 = name;
                        let ptr0 = vec0.as_ptr() as i32;
                        let len0 = vec0.len() as i32;
                        let ptr1 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]fields.get"]
                            fn wit_import(_: i32, _: i32, _: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0, len0, ptr1);
                        let l2 = *((ptr1 + 0) as *const i32);
                        let l3 = *((ptr1 + 4) as *const i32);
                        let base7 = l2;
                        let len7 = l3;
                        let mut result7 = Vec::with_capacity(len7 as usize);
                        for i in 0..len7 {
                            let base = base7 + i * 8;
                            let e7 = {
                                let l4 = *((base + 0) as *const i32);
                                let l5 = *((base + 4) as *const i32);
                                let len6 = l5 as usize;
                                Vec::from_raw_parts(l4 as *mut _, len6, len6)
                            };
                            result7.push(e7);
                        }
                        plugin::wit_bindgen::rt::dealloc(base7, (len7 as usize) * 8, 4);
                        result7
                    }
                }
            }
            impl Fields {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns `true` when the key is present in this `fields`. If the key is
                /// syntactically invalid, `false` is returned.
                pub fn has(&self, name: &FieldKey) -> bool {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        let vec0 = name;
                        let ptr0 = vec0.as_ptr() as i32;
                        let len0 = vec0.len() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]fields.has"]
                            fn wit_import(_: i32, _: i32, _: i32) -> i32;
                        }
                        let ret = wit_import((self).handle() as i32, ptr0, len0);
                        plugin::wit_bindgen::rt::bool_lift(ret as u8)
                    }
                }
            }
            impl Fields {
                #[allow(unused_unsafe, clippy::all)]
                /// Set all of the values for a key. Clears any existing values for that
                /// key, if they have been set.
                ///
                /// Fails with `header-error.immutable` if the `fields` are immutable.
                pub fn set(
                    &self,
                    name: &FieldKey,
                    value: &[FieldValue],
                ) -> Result<(), HeaderError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(1))]
                        struct RetArea([u8; 2]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let vec0 = name;
                        let ptr0 = vec0.as_ptr() as i32;
                        let len0 = vec0.len() as i32;
                        let vec2 = value;
                        let len2 = vec2.len() as i32;
                        let layout2 = alloc::Layout::from_size_align_unchecked(vec2.len() * 8, 4);
                        let result2 = if layout2.size() != 0 {
                            let ptr = alloc::alloc(layout2);
                            if ptr.is_null() {
                                alloc::handle_alloc_error(layout2);
                            }
                            ptr
                        } else {
                            {
                                ::core::ptr::null_mut()
                            }
                        };
                        for (i, e) in vec2.into_iter().enumerate() {
                            let base = result2 as i32 + (i as i32) * 8;
                            {
                                let vec1 = e;
                                let ptr1 = vec1.as_ptr() as i32;
                                let len1 = vec1.len() as i32;
                                *((base + 4) as *mut i32) = len1;
                                *((base + 0) as *mut i32) = ptr1;
                            }
                        }
                        let ptr3 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]fields.set"]
                            fn wit_import(_: i32, _: i32, _: i32, _: i32, _: i32, _: i32);
                        }
                        wit_import(
                            (self).handle() as i32,
                            ptr0,
                            len0,
                            result2 as i32,
                            len2,
                            ptr3,
                        );
                        let l4 = i32::from(*((ptr3 + 0) as *const u8));
                        if layout2.size() != 0 {
                            alloc::dealloc(result2, layout2);
                        }
                        match l4 {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l5 = i32::from(*((ptr3 + 1) as *const u8));
                                    let v6 = match l5 {
                                        0 => HeaderError::InvalidSyntax,
                                        1 => HeaderError::Forbidden,
                                        n => {
                                            if true {
                                                match (&n, &2) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            HeaderError::Immutable
                                        }
                                    };
                                    v6
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl Fields {
                #[allow(unused_unsafe, clippy::all)]
                /// Delete all values for a key. Does nothing if no values for the key
                /// exist.
                ///
                /// Fails with `header-error.immutable` if the `fields` are immutable.
                pub fn delete(&self, name: &FieldKey) -> Result<(), HeaderError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(1))]
                        struct RetArea([u8; 2]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let vec0 = name;
                        let ptr0 = vec0.as_ptr() as i32;
                        let len0 = vec0.len() as i32;
                        let ptr1 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]fields.delete"]
                            fn wit_import(_: i32, _: i32, _: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0, len0, ptr1);
                        let l2 = i32::from(*((ptr1 + 0) as *const u8));
                        match l2 {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l3 = i32::from(*((ptr1 + 1) as *const u8));
                                    let v4 = match l3 {
                                        0 => HeaderError::InvalidSyntax,
                                        1 => HeaderError::Forbidden,
                                        n => {
                                            if true {
                                                match (&n, &2) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            HeaderError::Immutable
                                        }
                                    };
                                    v4
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl Fields {
                #[allow(unused_unsafe, clippy::all)]
                /// Append a value for a key. Does not change or delete any existing
                /// values for that key.
                ///
                /// Fails with `header-error.immutable` if the `fields` are immutable.
                pub fn append(
                    &self,
                    name: &FieldKey,
                    value: &FieldValue,
                ) -> Result<(), HeaderError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(1))]
                        struct RetArea([u8; 2]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let vec0 = name;
                        let ptr0 = vec0.as_ptr() as i32;
                        let len0 = vec0.len() as i32;
                        let vec1 = value;
                        let ptr1 = vec1.as_ptr() as i32;
                        let len1 = vec1.len() as i32;
                        let ptr2 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]fields.append"]
                            fn wit_import(_: i32, _: i32, _: i32, _: i32, _: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0, len0, ptr1, len1, ptr2);
                        let l3 = i32::from(*((ptr2 + 0) as *const u8));
                        match l3 {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l4 = i32::from(*((ptr2 + 1) as *const u8));
                                    let v5 = match l4 {
                                        0 => HeaderError::InvalidSyntax,
                                        1 => HeaderError::Forbidden,
                                        n => {
                                            if true {
                                                match (&n, &2) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            HeaderError::Immutable
                                        }
                                    };
                                    v5
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl Fields {
                #[allow(unused_unsafe, clippy::all)]
                /// Retrieve the full set of keys and values in the Fields. Like the
                /// constructor, the list represents each key-value pair.
                ///
                /// The outer list represents each key-value pair in the Fields. Keys
                /// which have multiple values are represented by multiple entries in this
                /// list with the same key.
                pub fn entries(&self) -> plugin::wit_bindgen::rt::vec::Vec<(FieldKey, FieldValue)> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 8]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]fields.entries"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = *((ptr0 + 0) as *const i32);
                        let l2 = *((ptr0 + 4) as *const i32);
                        let base9 = l1;
                        let len9 = l2;
                        let mut result9 = Vec::with_capacity(len9 as usize);
                        for i in 0..len9 {
                            let base = base9 + i * 16;
                            let e9 = {
                                let l3 = *((base + 0) as *const i32);
                                let l4 = *((base + 4) as *const i32);
                                let len5 = l4 as usize;
                                let bytes5 = Vec::from_raw_parts(l3 as *mut _, len5, len5);
                                let l6 = *((base + 8) as *const i32);
                                let l7 = *((base + 12) as *const i32);
                                let len8 = l7 as usize;
                                (
                                    plugin::wit_bindgen::rt::string_lift(bytes5),
                                    Vec::from_raw_parts(l6 as *mut _, len8, len8),
                                )
                            };
                            result9.push(e9);
                        }
                        plugin::wit_bindgen::rt::dealloc(base9, (len9 as usize) * 16, 4);
                        result9
                    }
                }
            }
            impl Fields {
                #[allow(unused_unsafe, clippy::all)]
                /// Make a deep copy of the Fields. Equivelant in behavior to calling the
                /// `fields` constructor on the return value of `entries`. The resulting
                /// `fields` is mutable.
                pub fn clone(&self) -> Fields {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]fields.clone"]
                            fn wit_import(_: i32) -> i32;
                        }
                        let ret = wit_import((self).handle() as i32);
                        Fields::from_handle(ret as u32)
                    }
                }
            }
            impl IncomingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns the method of the incoming request.
                pub fn method(&self) -> Method {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 12]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]incoming-request.method"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        let v5 = match l1 {
                            0 => Method::Get,
                            1 => Method::Head,
                            2 => Method::Post,
                            3 => Method::Put,
                            4 => Method::Delete,
                            5 => Method::Connect,
                            6 => Method::Options,
                            7 => Method::Trace,
                            8 => Method::Patch,
                            n => {
                                if true {
                                    match (&n, &9) {
                                        (left_val, right_val) => {
                                            if !(*left_val == *right_val) {
                                                let kind = ::core::panicking::AssertKind::Eq;
                                                ::core::panicking::assert_failed(
                                                    kind,
                                                    &*left_val,
                                                    &*right_val,
                                                    ::core::option::Option::Some(format_args!(
                                                        "invalid enum discriminant"
                                                    )),
                                                );
                                            }
                                        }
                                    };
                                };
                                let e5 = {
                                    let l2 = *((ptr0 + 4) as *const i32);
                                    let l3 = *((ptr0 + 8) as *const i32);
                                    let len4 = l3 as usize;
                                    let bytes4 = Vec::from_raw_parts(l2 as *mut _, len4, len4);
                                    plugin::wit_bindgen::rt::string_lift(bytes4)
                                };
                                Method::Other(e5)
                            }
                        };
                        v5
                    }
                }
            }
            impl IncomingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns the path with query parameters from the request, as a string.
                pub fn path_with_query(&self) -> Option<plugin::wit_bindgen::rt::string::String> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 12]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]incoming-request.path-with-query"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => None,
                            1 => {
                                let e = {
                                    let l2 = *((ptr0 + 4) as *const i32);
                                    let l3 = *((ptr0 + 8) as *const i32);
                                    let len4 = l3 as usize;
                                    let bytes4 = Vec::from_raw_parts(l2 as *mut _, len4, len4);
                                    plugin::wit_bindgen::rt::string_lift(bytes4)
                                };
                                Some(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl IncomingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns the protocol scheme from the request.
                pub fn scheme(&self) -> Option<Scheme> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 16]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]incoming-request.scheme"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => None,
                            1 => {
                                let e = {
                                    let l2 = i32::from(*((ptr0 + 4) as *const u8));
                                    let v6 = match l2 {
                                        0 => Scheme::Http,
                                        1 => Scheme::Https,
                                        n => {
                                            if true {
                                                match (&n, &2) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            let e6 = {
                                                let l3 = *((ptr0 + 8) as *const i32);
                                                let l4 = *((ptr0 + 12) as *const i32);
                                                let len5 = l4 as usize;
                                                let bytes5 =
                                                    Vec::from_raw_parts(l3 as *mut _, len5, len5);
                                                plugin::wit_bindgen::rt::string_lift(bytes5)
                                            };
                                            Scheme::Other(e6)
                                        }
                                    };
                                    v6
                                };
                                Some(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl IncomingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns the authority from the request, if it was present.
                pub fn authority(&self) -> Option<plugin::wit_bindgen::rt::string::String> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 12]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]incoming-request.authority"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => None,
                            1 => {
                                let e = {
                                    let l2 = *((ptr0 + 4) as *const i32);
                                    let l3 = *((ptr0 + 8) as *const i32);
                                    let len4 = l3 as usize;
                                    let bytes4 = Vec::from_raw_parts(l2 as *mut _, len4, len4);
                                    plugin::wit_bindgen::rt::string_lift(bytes4)
                                };
                                Some(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl IncomingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Get the `headers` associated with the request.
                ///
                /// The returned `headers` resource is immutable: `set`, `append`, and
                /// `delete` operations will fail with `header-error.immutable`.
                ///
                /// The `headers` returned are a child resource: it must be dropped before
                /// the parent `incoming-request` is dropped. Dropping this
                /// `incoming-request` before all children are dropped will trap.
                pub fn headers(&self) -> Headers {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]incoming-request.headers"]
                            fn wit_import(_: i32) -> i32;
                        }
                        let ret = wit_import((self).handle() as i32);
                        Fields::from_handle(ret as u32)
                    }
                }
            }
            impl IncomingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Gives the `incoming-body` associated with this request. Will only
                /// return success at most once, and subsequent calls will return error.
                pub fn consume(&self) -> Result<IncomingBody, ()> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 8]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]incoming-request.consume"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = {
                                    let l2 = *((ptr0 + 4) as *const i32);
                                    IncomingBody::from_handle(l2 as u32)
                                };
                                Ok(e)
                            }
                            1 => {
                                let e = ();
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutgoingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Construct a new `outgoing-request` with a default `method` of `GET`, and
                /// `none` values for `path-with-query`, `scheme`, and `authority`.
                ///
                /// * `headers` is the HTTP Headers for the Request.
                ///
                /// It is possible to construct, or manipulate with the accessor functions
                /// below, an `outgoing-request` with an invalid combination of `scheme`
                /// and `authority`, or `headers` which are not permitted to be sent.
                /// It is the obligation of the `outgoing-handler.handle` implementation
                /// to reject invalid constructions of `outgoing-request`.
                pub fn new(headers: Headers) -> Self {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[constructor]outgoing-request"]
                            fn wit_import(_: i32) -> i32;
                        }
                        let ret = wit_import((headers).into_handle() as i32);
                        OutgoingRequest::from_handle(ret as u32)
                    }
                }
            }
            impl OutgoingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns the resource corresponding to the outgoing Body for this
                /// Request.
                ///
                /// Returns success on the first call: the `outgoing-body` resource for
                /// this `outgoing-request` can be retrieved at most once. Subsequent
                /// calls will return error.
                pub fn body(&self) -> Result<OutgoingBody, ()> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 8]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]outgoing-request.body"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = {
                                    let l2 = *((ptr0 + 4) as *const i32);
                                    OutgoingBody::from_handle(l2 as u32)
                                };
                                Ok(e)
                            }
                            1 => {
                                let e = ();
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutgoingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Get the Method for the Request.
                pub fn method(&self) -> Method {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 12]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]outgoing-request.method"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        let v5 = match l1 {
                            0 => Method::Get,
                            1 => Method::Head,
                            2 => Method::Post,
                            3 => Method::Put,
                            4 => Method::Delete,
                            5 => Method::Connect,
                            6 => Method::Options,
                            7 => Method::Trace,
                            8 => Method::Patch,
                            n => {
                                if true {
                                    match (&n, &9) {
                                        (left_val, right_val) => {
                                            if !(*left_val == *right_val) {
                                                let kind = ::core::panicking::AssertKind::Eq;
                                                ::core::panicking::assert_failed(
                                                    kind,
                                                    &*left_val,
                                                    &*right_val,
                                                    ::core::option::Option::Some(format_args!(
                                                        "invalid enum discriminant"
                                                    )),
                                                );
                                            }
                                        }
                                    };
                                };
                                let e5 = {
                                    let l2 = *((ptr0 + 4) as *const i32);
                                    let l3 = *((ptr0 + 8) as *const i32);
                                    let len4 = l3 as usize;
                                    let bytes4 = Vec::from_raw_parts(l2 as *mut _, len4, len4);
                                    plugin::wit_bindgen::rt::string_lift(bytes4)
                                };
                                Method::Other(e5)
                            }
                        };
                        v5
                    }
                }
            }
            impl OutgoingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Set the Method for the Request. Fails if the string present in a
                /// `method.other` argument is not a syntactically valid method.
                pub fn set_method(&self, method: &Method) -> Result<(), ()> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        let (result1_0, result1_1, result1_2) = match method {
                            Method::Get => (0i32, 0i32, 0i32),
                            Method::Head => (1i32, 0i32, 0i32),
                            Method::Post => (2i32, 0i32, 0i32),
                            Method::Put => (3i32, 0i32, 0i32),
                            Method::Delete => (4i32, 0i32, 0i32),
                            Method::Connect => (5i32, 0i32, 0i32),
                            Method::Options => (6i32, 0i32, 0i32),
                            Method::Trace => (7i32, 0i32, 0i32),
                            Method::Patch => (8i32, 0i32, 0i32),
                            Method::Other(e) => {
                                let vec0 = e;
                                let ptr0 = vec0.as_ptr() as i32;
                                let len0 = vec0.len() as i32;
                                (9i32, ptr0, len0)
                            }
                        };
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]outgoing-request.set-method"]
                            fn wit_import(_: i32, _: i32, _: i32, _: i32) -> i32;
                        }
                        let ret =
                            wit_import((self).handle() as i32, result1_0, result1_1, result1_2);
                        match ret {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = ();
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutgoingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Get the combination of the HTTP Path and Query for the Request.
                /// When `none`, this represents an empty Path and empty Query.
                pub fn path_with_query(&self) -> Option<plugin::wit_bindgen::rt::string::String> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 12]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]outgoing-request.path-with-query"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => None,
                            1 => {
                                let e = {
                                    let l2 = *((ptr0 + 4) as *const i32);
                                    let l3 = *((ptr0 + 8) as *const i32);
                                    let len4 = l3 as usize;
                                    let bytes4 = Vec::from_raw_parts(l2 as *mut _, len4, len4);
                                    plugin::wit_bindgen::rt::string_lift(bytes4)
                                };
                                Some(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutgoingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Set the combination of the HTTP Path and Query for the Request.
                /// When `none`, this represents an empty Path and empty Query. Fails is the
                /// string given is not a syntactically valid path and query uri component.
                pub fn set_path_with_query(&self, path_with_query: Option<&str>) -> Result<(), ()> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        let (result1_0, result1_1, result1_2) = match path_with_query {
                            Some(e) => {
                                let vec0 = e;
                                let ptr0 = vec0.as_ptr() as i32;
                                let len0 = vec0.len() as i32;
                                (1i32, ptr0, len0)
                            }
                            None => (0i32, 0i32, 0i32),
                        };
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]outgoing-request.set-path-with-query"]
                            fn wit_import(_: i32, _: i32, _: i32, _: i32) -> i32;
                        }
                        let ret =
                            wit_import((self).handle() as i32, result1_0, result1_1, result1_2);
                        match ret {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = ();
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutgoingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Get the HTTP Related Scheme for the Request. When `none`, the
                /// implementation may choose an appropriate default scheme.
                pub fn scheme(&self) -> Option<Scheme> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 16]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]outgoing-request.scheme"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => None,
                            1 => {
                                let e = {
                                    let l2 = i32::from(*((ptr0 + 4) as *const u8));
                                    let v6 = match l2 {
                                        0 => Scheme::Http,
                                        1 => Scheme::Https,
                                        n => {
                                            if true {
                                                match (&n, &2) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            let e6 = {
                                                let l3 = *((ptr0 + 8) as *const i32);
                                                let l4 = *((ptr0 + 12) as *const i32);
                                                let len5 = l4 as usize;
                                                let bytes5 =
                                                    Vec::from_raw_parts(l3 as *mut _, len5, len5);
                                                plugin::wit_bindgen::rt::string_lift(bytes5)
                                            };
                                            Scheme::Other(e6)
                                        }
                                    };
                                    v6
                                };
                                Some(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutgoingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Set the HTTP Related Scheme for the Request. When `none`, the
                /// implementation may choose an appropriate default scheme. Fails if the
                /// string given is not a syntactically valid uri scheme.
                pub fn set_scheme(&self, scheme: Option<&Scheme>) -> Result<(), ()> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        let (result2_0, result2_1, result2_2, result2_3) = match scheme {
                            Some(e) => {
                                let (result1_0, result1_1, result1_2) = match e {
                                    Scheme::Http => (0i32, 0i32, 0i32),
                                    Scheme::Https => (1i32, 0i32, 0i32),
                                    Scheme::Other(e) => {
                                        let vec0 = e;
                                        let ptr0 = vec0.as_ptr() as i32;
                                        let len0 = vec0.len() as i32;
                                        (2i32, ptr0, len0)
                                    }
                                };
                                (1i32, result1_0, result1_1, result1_2)
                            }
                            None => (0i32, 0i32, 0i32, 0i32),
                        };
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]outgoing-request.set-scheme"]
                            fn wit_import(_: i32, _: i32, _: i32, _: i32, _: i32) -> i32;
                        }
                        let ret = wit_import(
                            (self).handle() as i32,
                            result2_0,
                            result2_1,
                            result2_2,
                            result2_3,
                        );
                        match ret {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = ();
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutgoingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Get the HTTP Authority for the Request. A value of `none` may be used
                /// with Related Schemes which do not require an Authority. The HTTP and
                /// HTTPS schemes always require an authority.
                pub fn authority(&self) -> Option<plugin::wit_bindgen::rt::string::String> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 12]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]outgoing-request.authority"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => None,
                            1 => {
                                let e = {
                                    let l2 = *((ptr0 + 4) as *const i32);
                                    let l3 = *((ptr0 + 8) as *const i32);
                                    let len4 = l3 as usize;
                                    let bytes4 = Vec::from_raw_parts(l2 as *mut _, len4, len4);
                                    plugin::wit_bindgen::rt::string_lift(bytes4)
                                };
                                Some(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutgoingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Set the HTTP Authority for the Request. A value of `none` may be used
                /// with Related Schemes which do not require an Authority. The HTTP and
                /// HTTPS schemes always require an authority. Fails if the string given is
                /// not a syntactically valid uri authority.
                pub fn set_authority(&self, authority: Option<&str>) -> Result<(), ()> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        let (result1_0, result1_1, result1_2) = match authority {
                            Some(e) => {
                                let vec0 = e;
                                let ptr0 = vec0.as_ptr() as i32;
                                let len0 = vec0.len() as i32;
                                (1i32, ptr0, len0)
                            }
                            None => (0i32, 0i32, 0i32),
                        };
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]outgoing-request.set-authority"]
                            fn wit_import(_: i32, _: i32, _: i32, _: i32) -> i32;
                        }
                        let ret =
                            wit_import((self).handle() as i32, result1_0, result1_1, result1_2);
                        match ret {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = ();
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutgoingRequest {
                #[allow(unused_unsafe, clippy::all)]
                /// Get the headers associated with the Request.
                ///
                /// The returned `headers` resource is immutable: `set`, `append`, and
                /// `delete` operations will fail with `header-error.immutable`.
                ///
                /// This headers resource is a child: it must be dropped before the parent
                /// `outgoing-request` is dropped, or its ownership is transfered to
                /// another component by e.g. `outgoing-handler.handle`.
                pub fn headers(&self) -> Headers {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]outgoing-request.headers"]
                            fn wit_import(_: i32) -> i32;
                        }
                        let ret = wit_import((self).handle() as i32);
                        Fields::from_handle(ret as u32)
                    }
                }
            }
            impl RequestOptions {
                #[allow(unused_unsafe, clippy::all)]
                /// Construct a default `request-options` value.
                pub fn new() -> Self {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[constructor]request-options"]
                            fn wit_import() -> i32;
                        }
                        let ret = wit_import();
                        RequestOptions::from_handle(ret as u32)
                    }
                }
            }
            impl RequestOptions {
                #[allow(unused_unsafe, clippy::all)]
                /// The timeout for the initial connect to the HTTP Server.
                pub fn connect_timeout(&self) -> Option<Duration> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(8))]
                        struct RetArea([u8; 16]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]request-options.connect-timeout"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => None,
                            1 => {
                                let e = {
                                    let l2 = *((ptr0 + 8) as *const i64);
                                    l2 as u64
                                };
                                Some(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl RequestOptions {
                #[allow(unused_unsafe, clippy::all)]
                /// Set the timeout for the initial connect to the HTTP Server. An error
                /// return value indicates that this timeout is not supported.
                pub fn set_connect_timeout(&self, duration: Option<Duration>) -> Result<(), ()> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        let (result0_0, result0_1) = match duration {
                            Some(e) => (1i32, plugin::wit_bindgen::rt::as_i64(e)),
                            None => (0i32, 0i64),
                        };
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]request-options.set-connect-timeout"]
                            fn wit_import(_: i32, _: i32, _: i64) -> i32;
                        }
                        let ret = wit_import((self).handle() as i32, result0_0, result0_1);
                        match ret {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = ();
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl RequestOptions {
                #[allow(unused_unsafe, clippy::all)]
                /// The timeout for receiving the first byte of the Response body.
                pub fn first_byte_timeout(&self) -> Option<Duration> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(8))]
                        struct RetArea([u8; 16]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]request-options.first-byte-timeout"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => None,
                            1 => {
                                let e = {
                                    let l2 = *((ptr0 + 8) as *const i64);
                                    l2 as u64
                                };
                                Some(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl RequestOptions {
                #[allow(unused_unsafe, clippy::all)]
                /// Set the timeout for receiving the first byte of the Response body. An
                /// error return value indicates that this timeout is not supported.
                pub fn set_first_byte_timeout(&self, duration: Option<Duration>) -> Result<(), ()> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        let (result0_0, result0_1) = match duration {
                            Some(e) => (1i32, plugin::wit_bindgen::rt::as_i64(e)),
                            None => (0i32, 0i64),
                        };
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]request-options.set-first-byte-timeout"]
                            fn wit_import(_: i32, _: i32, _: i64) -> i32;
                        }
                        let ret = wit_import((self).handle() as i32, result0_0, result0_1);
                        match ret {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = ();
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl RequestOptions {
                #[allow(unused_unsafe, clippy::all)]
                /// The timeout for receiving subsequent chunks of bytes in the Response
                /// body stream.
                pub fn between_bytes_timeout(&self) -> Option<Duration> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(8))]
                        struct RetArea([u8; 16]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]request-options.between-bytes-timeout"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => None,
                            1 => {
                                let e = {
                                    let l2 = *((ptr0 + 8) as *const i64);
                                    l2 as u64
                                };
                                Some(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl RequestOptions {
                #[allow(unused_unsafe, clippy::all)]
                /// Set the timeout for receiving subsequent chunks of bytes in the Response
                /// body stream. An error return value indicates that this timeout is not
                /// supported.
                pub fn set_between_bytes_timeout(
                    &self,
                    duration: Option<Duration>,
                ) -> Result<(), ()> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        let (result0_0, result0_1) = match duration {
                            Some(e) => (1i32, plugin::wit_bindgen::rt::as_i64(e)),
                            None => (0i32, 0i64),
                        };
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]request-options.set-between-bytes-timeout"]
                            fn wit_import(_: i32, _: i32, _: i64) -> i32;
                        }
                        let ret = wit_import((self).handle() as i32, result0_0, result0_1);
                        match ret {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = ();
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl ResponseOutparam {
                #[allow(unused_unsafe, clippy::all)]
                /// Set the value of the `response-outparam` to either send a response,
                /// or indicate an error.
                ///
                /// This method consumes the `response-outparam` to ensure that it is
                /// called at most once. If it is never called, the implementation
                /// will respond with an error.
                ///
                /// The user may provide an `error` to `response` to allow the
                /// implementation determine how to respond with an HTTP error response.
                pub fn set(
                    param: ResponseOutparam,
                    response: Result<OutgoingResponse, &ErrorCode>,
                ) {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        let (
                            result38_0,
                            result38_1,
                            result38_2,
                            result38_3,
                            result38_4,
                            result38_5,
                            result38_6,
                            result38_7,
                        ) = match response {
                            Ok(e) => (
                                0i32,
                                (e).into_handle() as i32,
                                0i32,
                                0i64,
                                0i32,
                                0i32,
                                0i32,
                                0i32,
                            ),
                            Err(e) => {
                                let (
                                    result37_0,
                                    result37_1,
                                    result37_2,
                                    result37_3,
                                    result37_4,
                                    result37_5,
                                    result37_6,
                                ) = match e {
                                    ErrorCode::DnsTimeout => {
                                        (0i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::DnsError(e) => {
                                        let DnsErrorPayload {
                                            rcode: rcode0,
                                            info_code: info_code0,
                                        } = e;
                                        let (result2_0, result2_1, result2_2) = match rcode0 {
                                            Some(e) => {
                                                let vec1 = e;
                                                let ptr1 = vec1.as_ptr() as i32;
                                                let len1 = vec1.len() as i32;
                                                (1i32, ptr1, len1)
                                            }
                                            None => (0i32, 0i32, 0i32),
                                        };
                                        let (result3_0, result3_1) = match info_code0 {
                                            Some(e) => (1i32, plugin::wit_bindgen::rt::as_i32(e)),
                                            None => (0i32, 0i32),
                                        };
                                        (
                                            1i32,
                                            result2_0,
                                            i64::from(result2_1),
                                            result2_2,
                                            result3_0,
                                            result3_1,
                                            0i32,
                                        )
                                    }
                                    ErrorCode::DestinationNotFound => {
                                        (2i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::DestinationUnavailable => {
                                        (3i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::DestinationIpProhibited => {
                                        (4i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::DestinationIpUnroutable => {
                                        (5i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::ConnectionRefused => {
                                        (6i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::ConnectionTerminated => {
                                        (7i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::ConnectionTimeout => {
                                        (8i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::ConnectionReadTimeout => {
                                        (9i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::ConnectionWriteTimeout => {
                                        (10i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::ConnectionLimitReached => {
                                        (11i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::TlsProtocolError => {
                                        (12i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::TlsCertificateError => {
                                        (13i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::TlsAlertReceived(e) => {
                                        let TlsAlertReceivedPayload {
                                            alert_id: alert_id4,
                                            alert_message: alert_message4,
                                        } = e;
                                        let (result5_0, result5_1) = match alert_id4 {
                                            Some(e) => (1i32, plugin::wit_bindgen::rt::as_i32(e)),
                                            None => (0i32, 0i32),
                                        };
                                        let (result7_0, result7_1, result7_2) = match alert_message4
                                        {
                                            Some(e) => {
                                                let vec6 = e;
                                                let ptr6 = vec6.as_ptr() as i32;
                                                let len6 = vec6.len() as i32;
                                                (1i32, ptr6, len6)
                                            }
                                            None => (0i32, 0i32, 0i32),
                                        };
                                        (
                                            14i32,
                                            result5_0,
                                            i64::from(result5_1),
                                            result7_0,
                                            result7_1,
                                            result7_2,
                                            0i32,
                                        )
                                    }
                                    ErrorCode::HttpRequestDenied => {
                                        (15i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::HttpRequestLengthRequired => {
                                        (16i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::HttpRequestBodySize(e) => {
                                        let (result8_0, result8_1) = match e {
                                            Some(e) => (1i32, plugin::wit_bindgen::rt::as_i64(e)),
                                            None => (0i32, 0i64),
                                        };
                                        (17i32, result8_0, result8_1, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::HttpRequestMethodInvalid => {
                                        (18i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::HttpRequestUriInvalid => {
                                        (19i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::HttpRequestUriTooLong => {
                                        (20i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::HttpRequestHeaderSectionSize(e) => {
                                        let (result9_0, result9_1) = match e {
                                            Some(e) => (1i32, plugin::wit_bindgen::rt::as_i32(e)),
                                            None => (0i32, 0i32),
                                        };
                                        (
                                            21i32,
                                            result9_0,
                                            i64::from(result9_1),
                                            0i32,
                                            0i32,
                                            0i32,
                                            0i32,
                                        )
                                    }
                                    ErrorCode::HttpRequestHeaderSize(e) => {
                                        let (
                                            result14_0,
                                            result14_1,
                                            result14_2,
                                            result14_3,
                                            result14_4,
                                            result14_5,
                                        ) = match e {
                                            Some(e) => {
                                                let FieldSizePayload {
                                                    field_name: field_name10,
                                                    field_size: field_size10,
                                                } = e;
                                                let (result12_0, result12_1, result12_2) =
                                                    match field_name10 {
                                                        Some(e) => {
                                                            let vec11 = e;
                                                            let ptr11 = vec11.as_ptr() as i32;
                                                            let len11 = vec11.len() as i32;
                                                            (1i32, ptr11, len11)
                                                        }
                                                        None => (0i32, 0i32, 0i32),
                                                    };
                                                let (result13_0, result13_1) = match field_size10 {
                                                    Some(e) => {
                                                        (1i32, plugin::wit_bindgen::rt::as_i32(e))
                                                    }
                                                    None => (0i32, 0i32),
                                                };
                                                (
                                                    1i32, result12_0, result12_1, result12_2,
                                                    result13_0, result13_1,
                                                )
                                            }
                                            None => (0i32, 0i32, 0i32, 0i32, 0i32, 0i32),
                                        };
                                        (
                                            22i32,
                                            result14_0,
                                            i64::from(result14_1),
                                            result14_2,
                                            result14_3,
                                            result14_4,
                                            result14_5,
                                        )
                                    }
                                    ErrorCode::HttpRequestTrailerSectionSize(e) => {
                                        let (result15_0, result15_1) = match e {
                                            Some(e) => (1i32, plugin::wit_bindgen::rt::as_i32(e)),
                                            None => (0i32, 0i32),
                                        };
                                        (
                                            23i32,
                                            result15_0,
                                            i64::from(result15_1),
                                            0i32,
                                            0i32,
                                            0i32,
                                            0i32,
                                        )
                                    }
                                    ErrorCode::HttpRequestTrailerSize(e) => {
                                        let FieldSizePayload {
                                            field_name: field_name16,
                                            field_size: field_size16,
                                        } = e;
                                        let (result18_0, result18_1, result18_2) =
                                            match field_name16 {
                                                Some(e) => {
                                                    let vec17 = e;
                                                    let ptr17 = vec17.as_ptr() as i32;
                                                    let len17 = vec17.len() as i32;
                                                    (1i32, ptr17, len17)
                                                }
                                                None => (0i32, 0i32, 0i32),
                                            };
                                        let (result19_0, result19_1) = match field_size16 {
                                            Some(e) => (1i32, plugin::wit_bindgen::rt::as_i32(e)),
                                            None => (0i32, 0i32),
                                        };
                                        (
                                            24i32,
                                            result18_0,
                                            i64::from(result18_1),
                                            result18_2,
                                            result19_0,
                                            result19_1,
                                            0i32,
                                        )
                                    }
                                    ErrorCode::HttpResponseIncomplete => {
                                        (25i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::HttpResponseHeaderSectionSize(e) => {
                                        let (result20_0, result20_1) = match e {
                                            Some(e) => (1i32, plugin::wit_bindgen::rt::as_i32(e)),
                                            None => (0i32, 0i32),
                                        };
                                        (
                                            26i32,
                                            result20_0,
                                            i64::from(result20_1),
                                            0i32,
                                            0i32,
                                            0i32,
                                            0i32,
                                        )
                                    }
                                    ErrorCode::HttpResponseHeaderSize(e) => {
                                        let FieldSizePayload {
                                            field_name: field_name21,
                                            field_size: field_size21,
                                        } = e;
                                        let (result23_0, result23_1, result23_2) =
                                            match field_name21 {
                                                Some(e) => {
                                                    let vec22 = e;
                                                    let ptr22 = vec22.as_ptr() as i32;
                                                    let len22 = vec22.len() as i32;
                                                    (1i32, ptr22, len22)
                                                }
                                                None => (0i32, 0i32, 0i32),
                                            };
                                        let (result24_0, result24_1) = match field_size21 {
                                            Some(e) => (1i32, plugin::wit_bindgen::rt::as_i32(e)),
                                            None => (0i32, 0i32),
                                        };
                                        (
                                            27i32,
                                            result23_0,
                                            i64::from(result23_1),
                                            result23_2,
                                            result24_0,
                                            result24_1,
                                            0i32,
                                        )
                                    }
                                    ErrorCode::HttpResponseBodySize(e) => {
                                        let (result25_0, result25_1) = match e {
                                            Some(e) => (1i32, plugin::wit_bindgen::rt::as_i64(e)),
                                            None => (0i32, 0i64),
                                        };
                                        (28i32, result25_0, result25_1, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::HttpResponseTrailerSectionSize(e) => {
                                        let (result26_0, result26_1) = match e {
                                            Some(e) => (1i32, plugin::wit_bindgen::rt::as_i32(e)),
                                            None => (0i32, 0i32),
                                        };
                                        (
                                            29i32,
                                            result26_0,
                                            i64::from(result26_1),
                                            0i32,
                                            0i32,
                                            0i32,
                                            0i32,
                                        )
                                    }
                                    ErrorCode::HttpResponseTrailerSize(e) => {
                                        let FieldSizePayload {
                                            field_name: field_name27,
                                            field_size: field_size27,
                                        } = e;
                                        let (result29_0, result29_1, result29_2) =
                                            match field_name27 {
                                                Some(e) => {
                                                    let vec28 = e;
                                                    let ptr28 = vec28.as_ptr() as i32;
                                                    let len28 = vec28.len() as i32;
                                                    (1i32, ptr28, len28)
                                                }
                                                None => (0i32, 0i32, 0i32),
                                            };
                                        let (result30_0, result30_1) = match field_size27 {
                                            Some(e) => (1i32, plugin::wit_bindgen::rt::as_i32(e)),
                                            None => (0i32, 0i32),
                                        };
                                        (
                                            30i32,
                                            result29_0,
                                            i64::from(result29_1),
                                            result29_2,
                                            result30_0,
                                            result30_1,
                                            0i32,
                                        )
                                    }
                                    ErrorCode::HttpResponseTransferCoding(e) => {
                                        let (result32_0, result32_1, result32_2) = match e {
                                            Some(e) => {
                                                let vec31 = e;
                                                let ptr31 = vec31.as_ptr() as i32;
                                                let len31 = vec31.len() as i32;
                                                (1i32, ptr31, len31)
                                            }
                                            None => (0i32, 0i32, 0i32),
                                        };
                                        (
                                            31i32,
                                            result32_0,
                                            i64::from(result32_1),
                                            result32_2,
                                            0i32,
                                            0i32,
                                            0i32,
                                        )
                                    }
                                    ErrorCode::HttpResponseContentCoding(e) => {
                                        let (result34_0, result34_1, result34_2) = match e {
                                            Some(e) => {
                                                let vec33 = e;
                                                let ptr33 = vec33.as_ptr() as i32;
                                                let len33 = vec33.len() as i32;
                                                (1i32, ptr33, len33)
                                            }
                                            None => (0i32, 0i32, 0i32),
                                        };
                                        (
                                            32i32,
                                            result34_0,
                                            i64::from(result34_1),
                                            result34_2,
                                            0i32,
                                            0i32,
                                            0i32,
                                        )
                                    }
                                    ErrorCode::HttpResponseTimeout => {
                                        (33i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::HttpUpgradeFailed => {
                                        (34i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::HttpProtocolError => {
                                        (35i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::LoopDetected => {
                                        (36i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::ConfigurationError => {
                                        (37i32, 0i32, 0i64, 0i32, 0i32, 0i32, 0i32)
                                    }
                                    ErrorCode::InternalError(e) => {
                                        let (result36_0, result36_1, result36_2) = match e {
                                            Some(e) => {
                                                let vec35 = e;
                                                let ptr35 = vec35.as_ptr() as i32;
                                                let len35 = vec35.len() as i32;
                                                (1i32, ptr35, len35)
                                            }
                                            None => (0i32, 0i32, 0i32),
                                        };
                                        (
                                            38i32,
                                            result36_0,
                                            i64::from(result36_1),
                                            result36_2,
                                            0i32,
                                            0i32,
                                            0i32,
                                        )
                                    }
                                };
                                (
                                    1i32, result37_0, result37_1, result37_2, result37_3,
                                    result37_4, result37_5, result37_6,
                                )
                            }
                        };
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[static]response-outparam.set"]
                            fn wit_import(
                                _: i32,
                                _: i32,
                                _: i32,
                                _: i32,
                                _: i64,
                                _: i32,
                                _: i32,
                                _: i32,
                                _: i32,
                            );
                        }
                        wit_import(
                            (param).into_handle() as i32,
                            result38_0,
                            result38_1,
                            result38_2,
                            result38_3,
                            result38_4,
                            result38_5,
                            result38_6,
                            result38_7,
                        );
                    }
                }
            }
            impl IncomingResponse {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns the status code from the incoming response.
                pub fn status(&self) -> StatusCode {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]incoming-response.status"]
                            fn wit_import(_: i32) -> i32;
                        }
                        let ret = wit_import((self).handle() as i32);
                        ret as u16
                    }
                }
            }
            impl IncomingResponse {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns the headers from the incoming response.
                ///
                /// The returned `headers` resource is immutable: `set`, `append`, and
                /// `delete` operations will fail with `header-error.immutable`.
                ///
                /// This headers resource is a child: it must be dropped before the parent
                /// `incoming-response` is dropped.
                pub fn headers(&self) -> Headers {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]incoming-response.headers"]
                            fn wit_import(_: i32) -> i32;
                        }
                        let ret = wit_import((self).handle() as i32);
                        Fields::from_handle(ret as u32)
                    }
                }
            }
            impl IncomingResponse {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns the incoming body. May be called at most once. Returns error
                /// if called additional times.
                pub fn consume(&self) -> Result<IncomingBody, ()> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 8]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]incoming-response.consume"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = {
                                    let l2 = *((ptr0 + 4) as *const i32);
                                    IncomingBody::from_handle(l2 as u32)
                                };
                                Ok(e)
                            }
                            1 => {
                                let e = ();
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl IncomingBody {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns the contents of the body, as a stream of bytes.
                ///
                /// Returns success on first call: the stream representing the contents
                /// can be retrieved at most once. Subsequent calls will return error.
                ///
                /// The returned `input-stream` resource is a child: it must be dropped
                /// before the parent `incoming-body` is dropped, or consumed by
                /// `incoming-body.finish`.
                ///
                /// This invariant ensures that the implementation can determine whether
                /// the user is consuming the contents of the body, waiting on the
                /// `future-trailers` to be ready, or neither. This allows for network
                /// backpressure is to be applied when the user is consuming the body,
                /// and for that backpressure to not inhibit delivery of the trailers if
                /// the user does not read the entire body.
                pub fn stream(&self) -> Result<InputStream, ()> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 8]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]incoming-body.stream"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = {
                                    let l2 = *((ptr0 + 4) as *const i32);
                                    super::super::super::wasi::io::streams::InputStream::from_handle(
                                        l2 as u32,
                                    )
                                };
                                Ok(e)
                            }
                            1 => {
                                let e = ();
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl IncomingBody {
                #[allow(unused_unsafe, clippy::all)]
                /// Takes ownership of `incoming-body`, and returns a `future-trailers`.
                /// This function will trap if the `input-stream` child is still alive.
                pub fn finish(this: IncomingBody) -> FutureTrailers {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[static]incoming-body.finish"]
                            fn wit_import(_: i32) -> i32;
                        }
                        let ret = wit_import((this).into_handle() as i32);
                        FutureTrailers::from_handle(ret as u32)
                    }
                }
            }
            impl FutureTrailers {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns a pollable which becomes ready when either the trailers have
                /// been received, or an error has occured. When this pollable is ready,
                /// the `get` method will return `some`.
                pub fn subscribe(&self) -> Pollable {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]future-trailers.subscribe"]
                            fn wit_import(_: i32) -> i32;
                        }
                        let ret = wit_import((self).handle() as i32);
                        super::super::super::wasi::io::poll::Pollable::from_handle(ret as u32)
                    }
                }
            }
            impl FutureTrailers {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns the contents of the trailers, or an error which occured,
                /// once the future is ready.
                ///
                /// The outer `option` represents future readiness. Users can wait on this
                /// `option` to become `some` using the `subscribe` method.
                ///
                /// The outer `result` is used to retrieve the trailers or error at most
                /// once. It will be success on the first call in which the outer option
                /// is `some`, and error on subsequent calls.
                ///
                /// The inner `result` represents that either the HTTP Request or Response
                /// body, as well as any trailers, were received successfully, or that an
                /// error occured receiving them. The optional `trailers` indicates whether
                /// or not trailers were present in the body.
                ///
                /// When some `trailers` are returned by this method, the `trailers`
                /// resource is immutable, and a child. Use of the `set`, `append`, or
                /// `delete` methods will return an error, and the resource must be
                /// dropped before the parent `future-trailers` is dropped.
                pub fn get(&self) -> Option<Result<Result<Option<Trailers>, ErrorCode>, ()>> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(8))]
                        struct RetArea([u8; 56]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]future-trailers.get"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => None,
                            1 => {
                                let e = {
                                    let l2 = i32::from(*((ptr0 + 8) as *const u8));
                                    match l2 {
                                        0 => {
                                            let e = {
                                                let l3 = i32::from(*((ptr0 + 16) as *const u8));
                                                match l3 { 0 => { let e = { let l4 = i32 :: from (* ((ptr0 + 24) as * const u8)) ; match l4 { 0 => None , 1 => { let e = { let l5 = * ((ptr0 + 28) as * const i32) ; Fields :: from_handle (l5 as u32) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; Ok (e) } 1 => { let e = { let l6 = i32 :: from (* ((ptr0 + 24) as * const u8)) ; let v68 = match l6 { 0 => { ErrorCode :: DnsTimeout } 1 => { let e68 = { let l7 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; let l11 = i32 :: from (* ((ptr0 + 44) as * const u8)) ; DnsErrorPayload { rcode : match l7 { 0 => None , 1 => { let e = { let l8 = * ((ptr0 + 36) as * const i32) ; let l9 = * ((ptr0 + 40) as * const i32) ; let len10 = l9 as usize ; let bytes10 = Vec :: from_raw_parts (l8 as * mut _ , len10 , len10) ; plugin :: wit_bindgen :: rt :: string_lift (bytes10) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , info_code : match l11 { 0 => None , 1 => { let e = { let l12 = i32 :: from (* ((ptr0 + 46) as * const u16)) ; l12 as u16 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , } } ; ErrorCode :: DnsError (e68) } 2 => { ErrorCode :: DestinationNotFound } 3 => { ErrorCode :: DestinationUnavailable } 4 => { ErrorCode :: DestinationIpProhibited } 5 => { ErrorCode :: DestinationIpUnroutable } 6 => { ErrorCode :: ConnectionRefused } 7 => { ErrorCode :: ConnectionTerminated } 8 => { ErrorCode :: ConnectionTimeout } 9 => { ErrorCode :: ConnectionReadTimeout } 10 => { ErrorCode :: ConnectionWriteTimeout } 11 => { ErrorCode :: ConnectionLimitReached } 12 => { ErrorCode :: TlsProtocolError } 13 => { ErrorCode :: TlsCertificateError } 14 => { let e68 = { let l13 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; let l15 = i32 :: from (* ((ptr0 + 36) as * const u8)) ; TlsAlertReceivedPayload { alert_id : match l13 { 0 => None , 1 => { let e = { let l14 = i32 :: from (* ((ptr0 + 33) as * const u8)) ; l14 as u8 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , alert_message : match l15 { 0 => None , 1 => { let e = { let l16 = * ((ptr0 + 40) as * const i32) ; let l17 = * ((ptr0 + 44) as * const i32) ; let len18 = l17 as usize ; let bytes18 = Vec :: from_raw_parts (l16 as * mut _ , len18 , len18) ; plugin :: wit_bindgen :: rt :: string_lift (bytes18) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , } } ; ErrorCode :: TlsAlertReceived (e68) } 15 => { ErrorCode :: HttpRequestDenied } 16 => { ErrorCode :: HttpRequestLengthRequired } 17 => { let e68 = { let l19 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l19 { 0 => None , 1 => { let e = { let l20 = * ((ptr0 + 40) as * const i64) ; l20 as u64 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpRequestBodySize (e68) } 18 => { ErrorCode :: HttpRequestMethodInvalid } 19 => { ErrorCode :: HttpRequestUriInvalid } 20 => { ErrorCode :: HttpRequestUriTooLong } 21 => { let e68 = { let l21 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l21 { 0 => None , 1 => { let e = { let l22 = * ((ptr0 + 36) as * const i32) ; l22 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpRequestHeaderSectionSize (e68) } 22 => { let e68 = { let l23 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l23 { 0 => None , 1 => { let e = { let l24 = i32 :: from (* ((ptr0 + 36) as * const u8)) ; let l28 = i32 :: from (* ((ptr0 + 48) as * const u8)) ; FieldSizePayload { field_name : match l24 { 0 => None , 1 => { let e = { let l25 = * ((ptr0 + 40) as * const i32) ; let l26 = * ((ptr0 + 44) as * const i32) ; let len27 = l26 as usize ; let bytes27 = Vec :: from_raw_parts (l25 as * mut _ , len27 , len27) ; plugin :: wit_bindgen :: rt :: string_lift (bytes27) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l28 { 0 => None , 1 => { let e = { let l29 = * ((ptr0 + 52) as * const i32) ; l29 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , } } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpRequestHeaderSize (e68) } 23 => { let e68 = { let l30 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l30 { 0 => None , 1 => { let e = { let l31 = * ((ptr0 + 36) as * const i32) ; l31 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpRequestTrailerSectionSize (e68) } 24 => { let e68 = { let l32 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; let l36 = i32 :: from (* ((ptr0 + 44) as * const u8)) ; FieldSizePayload { field_name : match l32 { 0 => None , 1 => { let e = { let l33 = * ((ptr0 + 36) as * const i32) ; let l34 = * ((ptr0 + 40) as * const i32) ; let len35 = l34 as usize ; let bytes35 = Vec :: from_raw_parts (l33 as * mut _ , len35 , len35) ; plugin :: wit_bindgen :: rt :: string_lift (bytes35) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l36 { 0 => None , 1 => { let e = { let l37 = * ((ptr0 + 48) as * const i32) ; l37 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , } } ; ErrorCode :: HttpRequestTrailerSize (e68) } 25 => { ErrorCode :: HttpResponseIncomplete } 26 => { let e68 = { let l38 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l38 { 0 => None , 1 => { let e = { let l39 = * ((ptr0 + 36) as * const i32) ; l39 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpResponseHeaderSectionSize (e68) } 27 => { let e68 = { let l40 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; let l44 = i32 :: from (* ((ptr0 + 44) as * const u8)) ; FieldSizePayload { field_name : match l40 { 0 => None , 1 => { let e = { let l41 = * ((ptr0 + 36) as * const i32) ; let l42 = * ((ptr0 + 40) as * const i32) ; let len43 = l42 as usize ; let bytes43 = Vec :: from_raw_parts (l41 as * mut _ , len43 , len43) ; plugin :: wit_bindgen :: rt :: string_lift (bytes43) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l44 { 0 => None , 1 => { let e = { let l45 = * ((ptr0 + 48) as * const i32) ; l45 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , } } ; ErrorCode :: HttpResponseHeaderSize (e68) } 28 => { let e68 = { let l46 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l46 { 0 => None , 1 => { let e = { let l47 = * ((ptr0 + 40) as * const i64) ; l47 as u64 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpResponseBodySize (e68) } 29 => { let e68 = { let l48 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l48 { 0 => None , 1 => { let e = { let l49 = * ((ptr0 + 36) as * const i32) ; l49 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpResponseTrailerSectionSize (e68) } 30 => { let e68 = { let l50 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; let l54 = i32 :: from (* ((ptr0 + 44) as * const u8)) ; FieldSizePayload { field_name : match l50 { 0 => None , 1 => { let e = { let l51 = * ((ptr0 + 36) as * const i32) ; let l52 = * ((ptr0 + 40) as * const i32) ; let len53 = l52 as usize ; let bytes53 = Vec :: from_raw_parts (l51 as * mut _ , len53 , len53) ; plugin :: wit_bindgen :: rt :: string_lift (bytes53) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l54 { 0 => None , 1 => { let e = { let l55 = * ((ptr0 + 48) as * const i32) ; l55 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , } } ; ErrorCode :: HttpResponseTrailerSize (e68) } 31 => { let e68 = { let l56 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l56 { 0 => None , 1 => { let e = { let l57 = * ((ptr0 + 36) as * const i32) ; let l58 = * ((ptr0 + 40) as * const i32) ; let len59 = l58 as usize ; let bytes59 = Vec :: from_raw_parts (l57 as * mut _ , len59 , len59) ; plugin :: wit_bindgen :: rt :: string_lift (bytes59) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpResponseTransferCoding (e68) } 32 => { let e68 = { let l60 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l60 { 0 => None , 1 => { let e = { let l61 = * ((ptr0 + 36) as * const i32) ; let l62 = * ((ptr0 + 40) as * const i32) ; let len63 = l62 as usize ; let bytes63 = Vec :: from_raw_parts (l61 as * mut _ , len63 , len63) ; plugin :: wit_bindgen :: rt :: string_lift (bytes63) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpResponseContentCoding (e68) } 33 => { ErrorCode :: HttpResponseTimeout } 34 => { ErrorCode :: HttpUpgradeFailed } 35 => { ErrorCode :: HttpProtocolError } 36 => { ErrorCode :: LoopDetected } 37 => { ErrorCode :: ConfigurationError } n => { if true { match (& n , & 38) { (left_val , right_val) => { if ! (* left_val == * right_val) { let kind = :: core :: panicking :: AssertKind :: Eq ; :: core :: panicking :: assert_failed (kind , & * left_val , & * right_val , :: core :: option :: Option :: Some (format_args ! ("invalid enum discriminant"))) ; } } } ; } ; let e68 = { let l64 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l64 { 0 => None , 1 => { let e = { let l65 = * ((ptr0 + 36) as * const i32) ; let l66 = * ((ptr0 + 40) as * const i32) ; let len67 = l66 as usize ; let bytes67 = Vec :: from_raw_parts (l65 as * mut _ , len67 , len67) ; plugin :: wit_bindgen :: rt :: string_lift (bytes67) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: InternalError (e68) } } ; v68 } ; Err (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                            };
                                            Ok(e)
                                        }
                                        1 => {
                                            let e = ();
                                            Err(e)
                                        }
                                        _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                                    }
                                };
                                Some(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutgoingResponse {
                #[allow(unused_unsafe, clippy::all)]
                /// Construct an `outgoing-response`, with a default `status-code` of `200`.
                /// If a different `status-code` is needed, it must be set via the
                /// `set-status-code` method.
                ///
                /// * `headers` is the HTTP Headers for the Response.
                pub fn new(headers: Headers) -> Self {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[constructor]outgoing-response"]
                            fn wit_import(_: i32) -> i32;
                        }
                        let ret = wit_import((headers).into_handle() as i32);
                        OutgoingResponse::from_handle(ret as u32)
                    }
                }
            }
            impl OutgoingResponse {
                #[allow(unused_unsafe, clippy::all)]
                /// Get the HTTP Status Code for the Response.
                pub fn status_code(&self) -> StatusCode {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]outgoing-response.status-code"]
                            fn wit_import(_: i32) -> i32;
                        }
                        let ret = wit_import((self).handle() as i32);
                        ret as u16
                    }
                }
            }
            impl OutgoingResponse {
                #[allow(unused_unsafe, clippy::all)]
                /// Set the HTTP Status Code for the Response. Fails if the status-code
                /// given is not a valid http status code.
                pub fn set_status_code(&self, status_code: StatusCode) -> Result<(), ()> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]outgoing-response.set-status-code"]
                            fn wit_import(_: i32, _: i32) -> i32;
                        }
                        let ret = wit_import(
                            (self).handle() as i32,
                            plugin::wit_bindgen::rt::as_i32(status_code),
                        );
                        match ret {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = ();
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutgoingResponse {
                #[allow(unused_unsafe, clippy::all)]
                /// Get the headers associated with the Request.
                ///
                /// The returned `headers` resource is immutable: `set`, `append`, and
                /// `delete` operations will fail with `header-error.immutable`.
                ///
                /// This headers resource is a child: it must be dropped before the parent
                /// `outgoing-request` is dropped, or its ownership is transfered to
                /// another component by e.g. `outgoing-handler.handle`.
                pub fn headers(&self) -> Headers {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]outgoing-response.headers"]
                            fn wit_import(_: i32) -> i32;
                        }
                        let ret = wit_import((self).handle() as i32);
                        Fields::from_handle(ret as u32)
                    }
                }
            }
            impl OutgoingResponse {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns the resource corresponding to the outgoing Body for this Response.
                ///
                /// Returns success on the first call: the `outgoing-body` resource for
                /// this `outgoing-response` can be retrieved at most once. Subsequent
                /// calls will return error.
                pub fn body(&self) -> Result<OutgoingBody, ()> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 8]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]outgoing-response.body"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = {
                                    let l2 = *((ptr0 + 4) as *const i32);
                                    OutgoingBody::from_handle(l2 as u32)
                                };
                                Ok(e)
                            }
                            1 => {
                                let e = ();
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutgoingBody {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns a stream for writing the body contents.
                ///
                /// The returned `output-stream` is a child resource: it must be dropped
                /// before the parent `outgoing-body` resource is dropped (or finished),
                /// otherwise the `outgoing-body` drop or `finish` will trap.
                ///
                /// Returns success on the first call: the `output-stream` resource for
                /// this `outgoing-body` may be retrieved at most once. Subsequent calls
                /// will return error.
                pub fn write(&self) -> Result<OutputStream, ()> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 8]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]outgoing-body.write"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = {
                                    let l2 = *((ptr0 + 4) as *const i32);
                                    super :: super :: super :: wasi :: io :: streams :: OutputStream :: from_handle (l2 as u32)
                                };
                                Ok(e)
                            }
                            1 => {
                                let e = ();
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutgoingBody {
                #[allow(unused_unsafe, clippy::all)]
                /// Finalize an outgoing body, optionally providing trailers. This must be
                /// called to signal that the response is complete. If the `outgoing-body`
                /// is dropped without calling `outgoing-body.finalize`, the implementation
                /// should treat the body as corrupted.
                ///
                /// Fails if the body's `outgoing-request` or `outgoing-response` was
                /// constructed with a Content-Length header, and the contents written
                /// to the body (via `write`) does not match the value given in the
                /// Content-Length.
                pub fn finish(
                    this: OutgoingBody,
                    trailers: Option<Trailers>,
                ) -> Result<(), ErrorCode> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(8))]
                        struct RetArea([u8; 40]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let (result0_0, result0_1) = match trailers {
                            Some(e) => (1i32, (e).into_handle() as i32),
                            None => (0i32, 0i32),
                        };
                        let ptr1 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[static]outgoing-body.finish"]
                            fn wit_import(_: i32, _: i32, _: i32, _: i32);
                        }
                        wit_import((this).into_handle() as i32, result0_0, result0_1, ptr1);
                        let l2 = i32::from(*((ptr1 + 0) as *const u8));
                        match l2 {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l3 = i32::from(*((ptr1 + 8) as *const u8));
                                    let v65 = match l3 {
                                        0 => ErrorCode::DnsTimeout,
                                        1 => {
                                            let e65 = {
                                                let l4 = i32::from(*((ptr1 + 16) as *const u8));
                                                let l8 = i32::from(*((ptr1 + 28) as *const u8));
                                                DnsErrorPayload { rcode : match l4 { 0 => None , 1 => { let e = { let l5 = * ((ptr1 + 20) as * const i32) ; let l6 = * ((ptr1 + 24) as * const i32) ; let len7 = l6 as usize ; let bytes7 = Vec :: from_raw_parts (l5 as * mut _ , len7 , len7) ; plugin :: wit_bindgen :: rt :: string_lift (bytes7) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , info_code : match l8 { 0 => None , 1 => { let e = { let l9 = i32 :: from (* ((ptr1 + 30) as * const u16)) ; l9 as u16 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , }
                                            };
                                            ErrorCode::DnsError(e65)
                                        }
                                        2 => ErrorCode::DestinationNotFound,
                                        3 => ErrorCode::DestinationUnavailable,
                                        4 => ErrorCode::DestinationIpProhibited,
                                        5 => ErrorCode::DestinationIpUnroutable,
                                        6 => ErrorCode::ConnectionRefused,
                                        7 => ErrorCode::ConnectionTerminated,
                                        8 => ErrorCode::ConnectionTimeout,
                                        9 => ErrorCode::ConnectionReadTimeout,
                                        10 => ErrorCode::ConnectionWriteTimeout,
                                        11 => ErrorCode::ConnectionLimitReached,
                                        12 => ErrorCode::TlsProtocolError,
                                        13 => ErrorCode::TlsCertificateError,
                                        14 => {
                                            let e65 = {
                                                let l10 = i32::from(*((ptr1 + 16) as *const u8));
                                                let l12 = i32::from(*((ptr1 + 20) as *const u8));
                                                TlsAlertReceivedPayload { alert_id : match l10 { 0 => None , 1 => { let e = { let l11 = i32 :: from (* ((ptr1 + 17) as * const u8)) ; l11 as u8 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , alert_message : match l12 { 0 => None , 1 => { let e = { let l13 = * ((ptr1 + 24) as * const i32) ; let l14 = * ((ptr1 + 28) as * const i32) ; let len15 = l14 as usize ; let bytes15 = Vec :: from_raw_parts (l13 as * mut _ , len15 , len15) ; plugin :: wit_bindgen :: rt :: string_lift (bytes15) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , }
                                            };
                                            ErrorCode::TlsAlertReceived(e65)
                                        }
                                        15 => ErrorCode::HttpRequestDenied,
                                        16 => ErrorCode::HttpRequestLengthRequired,
                                        17 => {
                                            let e65 = {
                                                let l16 = i32::from(*((ptr1 + 16) as *const u8));
                                                match l16 { 0 => None , 1 => { let e = { let l17 = * ((ptr1 + 24) as * const i64) ; l17 as u64 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                            };
                                            ErrorCode::HttpRequestBodySize(e65)
                                        }
                                        18 => ErrorCode::HttpRequestMethodInvalid,
                                        19 => ErrorCode::HttpRequestUriInvalid,
                                        20 => ErrorCode::HttpRequestUriTooLong,
                                        21 => {
                                            let e65 = {
                                                let l18 = i32::from(*((ptr1 + 16) as *const u8));
                                                match l18 { 0 => None , 1 => { let e = { let l19 = * ((ptr1 + 20) as * const i32) ; l19 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                            };
                                            ErrorCode::HttpRequestHeaderSectionSize(e65)
                                        }
                                        22 => {
                                            let e65 = {
                                                let l20 = i32::from(*((ptr1 + 16) as *const u8));
                                                match l20 { 0 => None , 1 => { let e = { let l21 = i32 :: from (* ((ptr1 + 20) as * const u8)) ; let l25 = i32 :: from (* ((ptr1 + 32) as * const u8)) ; FieldSizePayload { field_name : match l21 { 0 => None , 1 => { let e = { let l22 = * ((ptr1 + 24) as * const i32) ; let l23 = * ((ptr1 + 28) as * const i32) ; let len24 = l23 as usize ; let bytes24 = Vec :: from_raw_parts (l22 as * mut _ , len24 , len24) ; plugin :: wit_bindgen :: rt :: string_lift (bytes24) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l25 { 0 => None , 1 => { let e = { let l26 = * ((ptr1 + 36) as * const i32) ; l26 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , } } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                            };
                                            ErrorCode::HttpRequestHeaderSize(e65)
                                        }
                                        23 => {
                                            let e65 = {
                                                let l27 = i32::from(*((ptr1 + 16) as *const u8));
                                                match l27 { 0 => None , 1 => { let e = { let l28 = * ((ptr1 + 20) as * const i32) ; l28 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                            };
                                            ErrorCode::HttpRequestTrailerSectionSize(e65)
                                        }
                                        24 => {
                                            let e65 = {
                                                let l29 = i32::from(*((ptr1 + 16) as *const u8));
                                                let l33 = i32::from(*((ptr1 + 28) as *const u8));
                                                FieldSizePayload { field_name : match l29 { 0 => None , 1 => { let e = { let l30 = * ((ptr1 + 20) as * const i32) ; let l31 = * ((ptr1 + 24) as * const i32) ; let len32 = l31 as usize ; let bytes32 = Vec :: from_raw_parts (l30 as * mut _ , len32 , len32) ; plugin :: wit_bindgen :: rt :: string_lift (bytes32) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l33 { 0 => None , 1 => { let e = { let l34 = * ((ptr1 + 32) as * const i32) ; l34 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , }
                                            };
                                            ErrorCode::HttpRequestTrailerSize(e65)
                                        }
                                        25 => ErrorCode::HttpResponseIncomplete,
                                        26 => {
                                            let e65 = {
                                                let l35 = i32::from(*((ptr1 + 16) as *const u8));
                                                match l35 { 0 => None , 1 => { let e = { let l36 = * ((ptr1 + 20) as * const i32) ; l36 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                            };
                                            ErrorCode::HttpResponseHeaderSectionSize(e65)
                                        }
                                        27 => {
                                            let e65 = {
                                                let l37 = i32::from(*((ptr1 + 16) as *const u8));
                                                let l41 = i32::from(*((ptr1 + 28) as *const u8));
                                                FieldSizePayload { field_name : match l37 { 0 => None , 1 => { let e = { let l38 = * ((ptr1 + 20) as * const i32) ; let l39 = * ((ptr1 + 24) as * const i32) ; let len40 = l39 as usize ; let bytes40 = Vec :: from_raw_parts (l38 as * mut _ , len40 , len40) ; plugin :: wit_bindgen :: rt :: string_lift (bytes40) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l41 { 0 => None , 1 => { let e = { let l42 = * ((ptr1 + 32) as * const i32) ; l42 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , }
                                            };
                                            ErrorCode::HttpResponseHeaderSize(e65)
                                        }
                                        28 => {
                                            let e65 = {
                                                let l43 = i32::from(*((ptr1 + 16) as *const u8));
                                                match l43 { 0 => None , 1 => { let e = { let l44 = * ((ptr1 + 24) as * const i64) ; l44 as u64 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                            };
                                            ErrorCode::HttpResponseBodySize(e65)
                                        }
                                        29 => {
                                            let e65 = {
                                                let l45 = i32::from(*((ptr1 + 16) as *const u8));
                                                match l45 { 0 => None , 1 => { let e = { let l46 = * ((ptr1 + 20) as * const i32) ; l46 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                            };
                                            ErrorCode::HttpResponseTrailerSectionSize(e65)
                                        }
                                        30 => {
                                            let e65 = {
                                                let l47 = i32::from(*((ptr1 + 16) as *const u8));
                                                let l51 = i32::from(*((ptr1 + 28) as *const u8));
                                                FieldSizePayload { field_name : match l47 { 0 => None , 1 => { let e = { let l48 = * ((ptr1 + 20) as * const i32) ; let l49 = * ((ptr1 + 24) as * const i32) ; let len50 = l49 as usize ; let bytes50 = Vec :: from_raw_parts (l48 as * mut _ , len50 , len50) ; plugin :: wit_bindgen :: rt :: string_lift (bytes50) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l51 { 0 => None , 1 => { let e = { let l52 = * ((ptr1 + 32) as * const i32) ; l52 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , }
                                            };
                                            ErrorCode::HttpResponseTrailerSize(e65)
                                        }
                                        31 => {
                                            let e65 = {
                                                let l53 = i32::from(*((ptr1 + 16) as *const u8));
                                                match l53 { 0 => None , 1 => { let e = { let l54 = * ((ptr1 + 20) as * const i32) ; let l55 = * ((ptr1 + 24) as * const i32) ; let len56 = l55 as usize ; let bytes56 = Vec :: from_raw_parts (l54 as * mut _ , len56 , len56) ; plugin :: wit_bindgen :: rt :: string_lift (bytes56) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                            };
                                            ErrorCode::HttpResponseTransferCoding(e65)
                                        }
                                        32 => {
                                            let e65 = {
                                                let l57 = i32::from(*((ptr1 + 16) as *const u8));
                                                match l57 { 0 => None , 1 => { let e = { let l58 = * ((ptr1 + 20) as * const i32) ; let l59 = * ((ptr1 + 24) as * const i32) ; let len60 = l59 as usize ; let bytes60 = Vec :: from_raw_parts (l58 as * mut _ , len60 , len60) ; plugin :: wit_bindgen :: rt :: string_lift (bytes60) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                            };
                                            ErrorCode::HttpResponseContentCoding(e65)
                                        }
                                        33 => ErrorCode::HttpResponseTimeout,
                                        34 => ErrorCode::HttpUpgradeFailed,
                                        35 => ErrorCode::HttpProtocolError,
                                        36 => ErrorCode::LoopDetected,
                                        37 => ErrorCode::ConfigurationError,
                                        n => {
                                            if true {
                                                match (&n, &38) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            let e65 = {
                                                let l61 = i32::from(*((ptr1 + 16) as *const u8));
                                                match l61 { 0 => None , 1 => { let e = { let l62 = * ((ptr1 + 20) as * const i32) ; let l63 = * ((ptr1 + 24) as * const i32) ; let len64 = l63 as usize ; let bytes64 = Vec :: from_raw_parts (l62 as * mut _ , len64 , len64) ; plugin :: wit_bindgen :: rt :: string_lift (bytes64) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                            };
                                            ErrorCode::InternalError(e65)
                                        }
                                    };
                                    v65
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl FutureIncomingResponse {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns a pollable which becomes ready when either the Response has
                /// been received, or an error has occured. When this pollable is ready,
                /// the `get` method will return `some`.
                pub fn subscribe(&self) -> Pollable {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]future-incoming-response.subscribe"]
                            fn wit_import(_: i32) -> i32;
                        }
                        let ret = wit_import((self).handle() as i32);
                        super::super::super::wasi::io::poll::Pollable::from_handle(ret as u32)
                    }
                }
            }
            impl FutureIncomingResponse {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns the incoming HTTP Response, or an error, once one is ready.
                ///
                /// The outer `option` represents future readiness. Users can wait on this
                /// `option` to become `some` using the `subscribe` method.
                ///
                /// The outer `result` is used to retrieve the response or error at most
                /// once. It will be success on the first call in which the outer option
                /// is `some`, and error on subsequent calls.
                ///
                /// The inner `result` represents that either the incoming HTTP Response
                /// status and headers have recieved successfully, or that an error
                /// occured. Errors may also occur while consuming the response body,
                /// but those will be reported by the `incoming-body` and its
                /// `output-stream` child.
                pub fn get(&self) -> Option<Result<Result<IncomingResponse, ErrorCode>, ()>> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(8))]
                        struct RetArea([u8; 56]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:http/types@0.2.0-rc-2023-12-05")]
                        extern "C" {
                            #[link_name = "[method]future-incoming-response.get"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => None,
                            1 => {
                                let e = {
                                    let l2 = i32::from(*((ptr0 + 8) as *const u8));
                                    match l2 {
                                        0 => {
                                            let e = {
                                                let l3 = i32::from(*((ptr0 + 16) as *const u8));
                                                match l3 { 0 => { let e = { let l4 = * ((ptr0 + 24) as * const i32) ; IncomingResponse :: from_handle (l4 as u32) } ; Ok (e) } 1 => { let e = { let l5 = i32 :: from (* ((ptr0 + 24) as * const u8)) ; let v67 = match l5 { 0 => { ErrorCode :: DnsTimeout } 1 => { let e67 = { let l6 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; let l10 = i32 :: from (* ((ptr0 + 44) as * const u8)) ; DnsErrorPayload { rcode : match l6 { 0 => None , 1 => { let e = { let l7 = * ((ptr0 + 36) as * const i32) ; let l8 = * ((ptr0 + 40) as * const i32) ; let len9 = l8 as usize ; let bytes9 = Vec :: from_raw_parts (l7 as * mut _ , len9 , len9) ; plugin :: wit_bindgen :: rt :: string_lift (bytes9) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , info_code : match l10 { 0 => None , 1 => { let e = { let l11 = i32 :: from (* ((ptr0 + 46) as * const u16)) ; l11 as u16 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , } } ; ErrorCode :: DnsError (e67) } 2 => { ErrorCode :: DestinationNotFound } 3 => { ErrorCode :: DestinationUnavailable } 4 => { ErrorCode :: DestinationIpProhibited } 5 => { ErrorCode :: DestinationIpUnroutable } 6 => { ErrorCode :: ConnectionRefused } 7 => { ErrorCode :: ConnectionTerminated } 8 => { ErrorCode :: ConnectionTimeout } 9 => { ErrorCode :: ConnectionReadTimeout } 10 => { ErrorCode :: ConnectionWriteTimeout } 11 => { ErrorCode :: ConnectionLimitReached } 12 => { ErrorCode :: TlsProtocolError } 13 => { ErrorCode :: TlsCertificateError } 14 => { let e67 = { let l12 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; let l14 = i32 :: from (* ((ptr0 + 36) as * const u8)) ; TlsAlertReceivedPayload { alert_id : match l12 { 0 => None , 1 => { let e = { let l13 = i32 :: from (* ((ptr0 + 33) as * const u8)) ; l13 as u8 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , alert_message : match l14 { 0 => None , 1 => { let e = { let l15 = * ((ptr0 + 40) as * const i32) ; let l16 = * ((ptr0 + 44) as * const i32) ; let len17 = l16 as usize ; let bytes17 = Vec :: from_raw_parts (l15 as * mut _ , len17 , len17) ; plugin :: wit_bindgen :: rt :: string_lift (bytes17) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , } } ; ErrorCode :: TlsAlertReceived (e67) } 15 => { ErrorCode :: HttpRequestDenied } 16 => { ErrorCode :: HttpRequestLengthRequired } 17 => { let e67 = { let l18 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l18 { 0 => None , 1 => { let e = { let l19 = * ((ptr0 + 40) as * const i64) ; l19 as u64 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpRequestBodySize (e67) } 18 => { ErrorCode :: HttpRequestMethodInvalid } 19 => { ErrorCode :: HttpRequestUriInvalid } 20 => { ErrorCode :: HttpRequestUriTooLong } 21 => { let e67 = { let l20 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l20 { 0 => None , 1 => { let e = { let l21 = * ((ptr0 + 36) as * const i32) ; l21 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpRequestHeaderSectionSize (e67) } 22 => { let e67 = { let l22 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l22 { 0 => None , 1 => { let e = { let l23 = i32 :: from (* ((ptr0 + 36) as * const u8)) ; let l27 = i32 :: from (* ((ptr0 + 48) as * const u8)) ; FieldSizePayload { field_name : match l23 { 0 => None , 1 => { let e = { let l24 = * ((ptr0 + 40) as * const i32) ; let l25 = * ((ptr0 + 44) as * const i32) ; let len26 = l25 as usize ; let bytes26 = Vec :: from_raw_parts (l24 as * mut _ , len26 , len26) ; plugin :: wit_bindgen :: rt :: string_lift (bytes26) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l27 { 0 => None , 1 => { let e = { let l28 = * ((ptr0 + 52) as * const i32) ; l28 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , } } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpRequestHeaderSize (e67) } 23 => { let e67 = { let l29 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l29 { 0 => None , 1 => { let e = { let l30 = * ((ptr0 + 36) as * const i32) ; l30 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpRequestTrailerSectionSize (e67) } 24 => { let e67 = { let l31 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; let l35 = i32 :: from (* ((ptr0 + 44) as * const u8)) ; FieldSizePayload { field_name : match l31 { 0 => None , 1 => { let e = { let l32 = * ((ptr0 + 36) as * const i32) ; let l33 = * ((ptr0 + 40) as * const i32) ; let len34 = l33 as usize ; let bytes34 = Vec :: from_raw_parts (l32 as * mut _ , len34 , len34) ; plugin :: wit_bindgen :: rt :: string_lift (bytes34) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l35 { 0 => None , 1 => { let e = { let l36 = * ((ptr0 + 48) as * const i32) ; l36 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , } } ; ErrorCode :: HttpRequestTrailerSize (e67) } 25 => { ErrorCode :: HttpResponseIncomplete } 26 => { let e67 = { let l37 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l37 { 0 => None , 1 => { let e = { let l38 = * ((ptr0 + 36) as * const i32) ; l38 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpResponseHeaderSectionSize (e67) } 27 => { let e67 = { let l39 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; let l43 = i32 :: from (* ((ptr0 + 44) as * const u8)) ; FieldSizePayload { field_name : match l39 { 0 => None , 1 => { let e = { let l40 = * ((ptr0 + 36) as * const i32) ; let l41 = * ((ptr0 + 40) as * const i32) ; let len42 = l41 as usize ; let bytes42 = Vec :: from_raw_parts (l40 as * mut _ , len42 , len42) ; plugin :: wit_bindgen :: rt :: string_lift (bytes42) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l43 { 0 => None , 1 => { let e = { let l44 = * ((ptr0 + 48) as * const i32) ; l44 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , } } ; ErrorCode :: HttpResponseHeaderSize (e67) } 28 => { let e67 = { let l45 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l45 { 0 => None , 1 => { let e = { let l46 = * ((ptr0 + 40) as * const i64) ; l46 as u64 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpResponseBodySize (e67) } 29 => { let e67 = { let l47 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l47 { 0 => None , 1 => { let e = { let l48 = * ((ptr0 + 36) as * const i32) ; l48 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpResponseTrailerSectionSize (e67) } 30 => { let e67 = { let l49 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; let l53 = i32 :: from (* ((ptr0 + 44) as * const u8)) ; FieldSizePayload { field_name : match l49 { 0 => None , 1 => { let e = { let l50 = * ((ptr0 + 36) as * const i32) ; let l51 = * ((ptr0 + 40) as * const i32) ; let len52 = l51 as usize ; let bytes52 = Vec :: from_raw_parts (l50 as * mut _ , len52 , len52) ; plugin :: wit_bindgen :: rt :: string_lift (bytes52) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l53 { 0 => None , 1 => { let e = { let l54 = * ((ptr0 + 48) as * const i32) ; l54 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , } } ; ErrorCode :: HttpResponseTrailerSize (e67) } 31 => { let e67 = { let l55 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l55 { 0 => None , 1 => { let e = { let l56 = * ((ptr0 + 36) as * const i32) ; let l57 = * ((ptr0 + 40) as * const i32) ; let len58 = l57 as usize ; let bytes58 = Vec :: from_raw_parts (l56 as * mut _ , len58 , len58) ; plugin :: wit_bindgen :: rt :: string_lift (bytes58) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpResponseTransferCoding (e67) } 32 => { let e67 = { let l59 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l59 { 0 => None , 1 => { let e = { let l60 = * ((ptr0 + 36) as * const i32) ; let l61 = * ((ptr0 + 40) as * const i32) ; let len62 = l61 as usize ; let bytes62 = Vec :: from_raw_parts (l60 as * mut _ , len62 , len62) ; plugin :: wit_bindgen :: rt :: string_lift (bytes62) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: HttpResponseContentCoding (e67) } 33 => { ErrorCode :: HttpResponseTimeout } 34 => { ErrorCode :: HttpUpgradeFailed } 35 => { ErrorCode :: HttpProtocolError } 36 => { ErrorCode :: LoopDetected } 37 => { ErrorCode :: ConfigurationError } n => { if true { match (& n , & 38) { (left_val , right_val) => { if ! (* left_val == * right_val) { let kind = :: core :: panicking :: AssertKind :: Eq ; :: core :: panicking :: assert_failed (kind , & * left_val , & * right_val , :: core :: option :: Option :: Some (format_args ! ("invalid enum discriminant"))) ; } } } ; } ; let e67 = { let l63 = i32 :: from (* ((ptr0 + 32) as * const u8)) ; match l63 { 0 => None , 1 => { let e = { let l64 = * ((ptr0 + 36) as * const i32) ; let l65 = * ((ptr0 + 40) as * const i32) ; let len66 = l65 as usize ; let bytes66 = Vec :: from_raw_parts (l64 as * mut _ , len66 , len66) ; plugin :: wit_bindgen :: rt :: string_lift (bytes66) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } } ; ErrorCode :: InternalError (e67) } } ; v67 } ; Err (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                            };
                                            Ok(e)
                                        }
                                        1 => {
                                            let e = ();
                                            Err(e)
                                        }
                                        _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                                    }
                                };
                                Some(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
        }
        #[allow(clippy::all)]
        pub mod outgoing_handler {
            #[used]
            #[doc(hidden)]
            #[cfg(target_arch = "wasm32")]
            static __FORCE_SECTION_REF: fn() = super::super::super::__link_section;
            pub type OutgoingRequest = super::super::super::wasi::http::types::OutgoingRequest;
            pub type RequestOptions = super::super::super::wasi::http::types::RequestOptions;
            pub type FutureIncomingResponse =
                super::super::super::wasi::http::types::FutureIncomingResponse;
            pub type ErrorCode = super::super::super::wasi::http::types::ErrorCode;
            #[allow(unused_unsafe, clippy::all)]
            /// This function is invoked with an outgoing HTTP Request, and it returns
            /// a resource `future-incoming-response` which represents an HTTP Response
            /// which may arrive in the future.
            ///
            /// The `options` argument accepts optional parameters for the HTTP
            /// protocol's transport layer.
            ///
            /// This function may return an error if the `outgoing-request` is invalid
            /// or not allowed to be made. Otherwise, protocol errors are reported
            /// through the `future-incoming-response`.
            pub fn handle(
                request: OutgoingRequest,
                options: Option<RequestOptions>,
            ) -> Result<FutureIncomingResponse, ErrorCode> {
                #[allow(unused_imports)]
                use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                unsafe {
                    #[repr(align(8))]
                    struct RetArea([u8; 40]);
                    let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                    let (result0_0, result0_1) = match options {
                        Some(e) => (1i32, (e).into_handle() as i32),
                        None => (0i32, 0i32),
                    };
                    let ptr1 = ret_area.as_mut_ptr() as i32;
                    #[cfg(target_arch = "wasm32")]
                    #[link(wasm_import_module = "wasi:http/outgoing-handler@0.2.0-rc-2023-12-05")]
                    extern "C" {
                        #[link_name = "handle"]
                        fn wit_import(_: i32, _: i32, _: i32, _: i32);
                    }
                    wit_import((request).into_handle() as i32, result0_0, result0_1, ptr1);
                    let l2 = i32::from(*((ptr1 + 0) as *const u8));
                    match l2 {
                        0 => {
                            let e = {
                                let l3 = *((ptr1 + 8) as *const i32);
                                super :: super :: super :: wasi :: http :: types :: FutureIncomingResponse :: from_handle (l3 as u32)
                            };
                            Ok(e)
                        }
                        1 => {
                            let e = {
                                let l4 = i32::from(*((ptr1 + 8) as *const u8));
                                use super::super::super::wasi::http::types::ErrorCode as V66;
                                let v66 = match l4 {
                                    0 => V66::DnsTimeout,
                                    1 => {
                                        let e66 = {
                                            let l5 = i32::from(*((ptr1 + 16) as *const u8));
                                            let l9 = i32::from(*((ptr1 + 28) as *const u8));
                                            super :: super :: super :: wasi :: http :: types :: DnsErrorPayload { rcode : match l5 { 0 => None , 1 => { let e = { let l6 = * ((ptr1 + 20) as * const i32) ; let l7 = * ((ptr1 + 24) as * const i32) ; let len8 = l7 as usize ; let bytes8 = Vec :: from_raw_parts (l6 as * mut _ , len8 , len8) ; plugin :: wit_bindgen :: rt :: string_lift (bytes8) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , info_code : match l9 { 0 => None , 1 => { let e = { let l10 = i32 :: from (* ((ptr1 + 30) as * const u16)) ; l10 as u16 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , }
                                        };
                                        V66::DnsError(e66)
                                    }
                                    2 => V66::DestinationNotFound,
                                    3 => V66::DestinationUnavailable,
                                    4 => V66::DestinationIpProhibited,
                                    5 => V66::DestinationIpUnroutable,
                                    6 => V66::ConnectionRefused,
                                    7 => V66::ConnectionTerminated,
                                    8 => V66::ConnectionTimeout,
                                    9 => V66::ConnectionReadTimeout,
                                    10 => V66::ConnectionWriteTimeout,
                                    11 => V66::ConnectionLimitReached,
                                    12 => V66::TlsProtocolError,
                                    13 => V66::TlsCertificateError,
                                    14 => {
                                        let e66 = {
                                            let l11 = i32::from(*((ptr1 + 16) as *const u8));
                                            let l13 = i32::from(*((ptr1 + 20) as *const u8));
                                            super :: super :: super :: wasi :: http :: types :: TlsAlertReceivedPayload { alert_id : match l11 { 0 => None , 1 => { let e = { let l12 = i32 :: from (* ((ptr1 + 17) as * const u8)) ; l12 as u8 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , alert_message : match l13 { 0 => None , 1 => { let e = { let l14 = * ((ptr1 + 24) as * const i32) ; let l15 = * ((ptr1 + 28) as * const i32) ; let len16 = l15 as usize ; let bytes16 = Vec :: from_raw_parts (l14 as * mut _ , len16 , len16) ; plugin :: wit_bindgen :: rt :: string_lift (bytes16) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , }
                                        };
                                        V66::TlsAlertReceived(e66)
                                    }
                                    15 => V66::HttpRequestDenied,
                                    16 => V66::HttpRequestLengthRequired,
                                    17 => {
                                        let e66 = {
                                            let l17 = i32::from(*((ptr1 + 16) as *const u8));
                                            match l17 { 0 => None , 1 => { let e = { let l18 = * ((ptr1 + 24) as * const i64) ; l18 as u64 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        V66::HttpRequestBodySize(e66)
                                    }
                                    18 => V66::HttpRequestMethodInvalid,
                                    19 => V66::HttpRequestUriInvalid,
                                    20 => V66::HttpRequestUriTooLong,
                                    21 => {
                                        let e66 = {
                                            let l19 = i32::from(*((ptr1 + 16) as *const u8));
                                            match l19 { 0 => None , 1 => { let e = { let l20 = * ((ptr1 + 20) as * const i32) ; l20 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        V66::HttpRequestHeaderSectionSize(e66)
                                    }
                                    22 => {
                                        let e66 = {
                                            let l21 = i32::from(*((ptr1 + 16) as *const u8));
                                            match l21 { 0 => None , 1 => { let e = { let l22 = i32 :: from (* ((ptr1 + 20) as * const u8)) ; let l26 = i32 :: from (* ((ptr1 + 32) as * const u8)) ; super :: super :: super :: wasi :: http :: types :: FieldSizePayload { field_name : match l22 { 0 => None , 1 => { let e = { let l23 = * ((ptr1 + 24) as * const i32) ; let l24 = * ((ptr1 + 28) as * const i32) ; let len25 = l24 as usize ; let bytes25 = Vec :: from_raw_parts (l23 as * mut _ , len25 , len25) ; plugin :: wit_bindgen :: rt :: string_lift (bytes25) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l26 { 0 => None , 1 => { let e = { let l27 = * ((ptr1 + 36) as * const i32) ; l27 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , } } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        V66::HttpRequestHeaderSize(e66)
                                    }
                                    23 => {
                                        let e66 = {
                                            let l28 = i32::from(*((ptr1 + 16) as *const u8));
                                            match l28 { 0 => None , 1 => { let e = { let l29 = * ((ptr1 + 20) as * const i32) ; l29 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        V66::HttpRequestTrailerSectionSize(e66)
                                    }
                                    24 => {
                                        let e66 = {
                                            let l30 = i32::from(*((ptr1 + 16) as *const u8));
                                            let l34 = i32::from(*((ptr1 + 28) as *const u8));
                                            super :: super :: super :: wasi :: http :: types :: FieldSizePayload { field_name : match l30 { 0 => None , 1 => { let e = { let l31 = * ((ptr1 + 20) as * const i32) ; let l32 = * ((ptr1 + 24) as * const i32) ; let len33 = l32 as usize ; let bytes33 = Vec :: from_raw_parts (l31 as * mut _ , len33 , len33) ; plugin :: wit_bindgen :: rt :: string_lift (bytes33) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l34 { 0 => None , 1 => { let e = { let l35 = * ((ptr1 + 32) as * const i32) ; l35 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , }
                                        };
                                        V66::HttpRequestTrailerSize(e66)
                                    }
                                    25 => V66::HttpResponseIncomplete,
                                    26 => {
                                        let e66 = {
                                            let l36 = i32::from(*((ptr1 + 16) as *const u8));
                                            match l36 { 0 => None , 1 => { let e = { let l37 = * ((ptr1 + 20) as * const i32) ; l37 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        V66::HttpResponseHeaderSectionSize(e66)
                                    }
                                    27 => {
                                        let e66 = {
                                            let l38 = i32::from(*((ptr1 + 16) as *const u8));
                                            let l42 = i32::from(*((ptr1 + 28) as *const u8));
                                            super :: super :: super :: wasi :: http :: types :: FieldSizePayload { field_name : match l38 { 0 => None , 1 => { let e = { let l39 = * ((ptr1 + 20) as * const i32) ; let l40 = * ((ptr1 + 24) as * const i32) ; let len41 = l40 as usize ; let bytes41 = Vec :: from_raw_parts (l39 as * mut _ , len41 , len41) ; plugin :: wit_bindgen :: rt :: string_lift (bytes41) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l42 { 0 => None , 1 => { let e = { let l43 = * ((ptr1 + 32) as * const i32) ; l43 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , }
                                        };
                                        V66::HttpResponseHeaderSize(e66)
                                    }
                                    28 => {
                                        let e66 = {
                                            let l44 = i32::from(*((ptr1 + 16) as *const u8));
                                            match l44 { 0 => None , 1 => { let e = { let l45 = * ((ptr1 + 24) as * const i64) ; l45 as u64 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        V66::HttpResponseBodySize(e66)
                                    }
                                    29 => {
                                        let e66 = {
                                            let l46 = i32::from(*((ptr1 + 16) as *const u8));
                                            match l46 { 0 => None , 1 => { let e = { let l47 = * ((ptr1 + 20) as * const i32) ; l47 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        V66::HttpResponseTrailerSectionSize(e66)
                                    }
                                    30 => {
                                        let e66 = {
                                            let l48 = i32::from(*((ptr1 + 16) as *const u8));
                                            let l52 = i32::from(*((ptr1 + 28) as *const u8));
                                            super :: super :: super :: wasi :: http :: types :: FieldSizePayload { field_name : match l48 { 0 => None , 1 => { let e = { let l49 = * ((ptr1 + 20) as * const i32) ; let l50 = * ((ptr1 + 24) as * const i32) ; let len51 = l50 as usize ; let bytes51 = Vec :: from_raw_parts (l49 as * mut _ , len51 , len51) ; plugin :: wit_bindgen :: rt :: string_lift (bytes51) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , field_size : match l52 { 0 => None , 1 => { let e = { let l53 = * ((ptr1 + 32) as * const i32) ; l53 as u32 } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , } , }
                                        };
                                        V66::HttpResponseTrailerSize(e66)
                                    }
                                    31 => {
                                        let e66 = {
                                            let l54 = i32::from(*((ptr1 + 16) as *const u8));
                                            match l54 { 0 => None , 1 => { let e = { let l55 = * ((ptr1 + 20) as * const i32) ; let l56 = * ((ptr1 + 24) as * const i32) ; let len57 = l56 as usize ; let bytes57 = Vec :: from_raw_parts (l55 as * mut _ , len57 , len57) ; plugin :: wit_bindgen :: rt :: string_lift (bytes57) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        V66::HttpResponseTransferCoding(e66)
                                    }
                                    32 => {
                                        let e66 = {
                                            let l58 = i32::from(*((ptr1 + 16) as *const u8));
                                            match l58 { 0 => None , 1 => { let e = { let l59 = * ((ptr1 + 20) as * const i32) ; let l60 = * ((ptr1 + 24) as * const i32) ; let len61 = l60 as usize ; let bytes61 = Vec :: from_raw_parts (l59 as * mut _ , len61 , len61) ; plugin :: wit_bindgen :: rt :: string_lift (bytes61) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        V66::HttpResponseContentCoding(e66)
                                    }
                                    33 => V66::HttpResponseTimeout,
                                    34 => V66::HttpUpgradeFailed,
                                    35 => V66::HttpProtocolError,
                                    36 => V66::LoopDetected,
                                    37 => V66::ConfigurationError,
                                    n => {
                                        if true {
                                            match (&n, &38) {
                                                (left_val, right_val) => {
                                                    if !(*left_val == *right_val) {
                                                        let kind =
                                                            ::core::panicking::AssertKind::Eq;
                                                        ::core::panicking::assert_failed(
                                                            kind,
                                                            &*left_val,
                                                            &*right_val,
                                                            ::core::option::Option::Some(
                                                                format_args!(
                                                                    "invalid enum discriminant"
                                                                ),
                                                            ),
                                                        );
                                                    }
                                                }
                                            };
                                        };
                                        let e66 = {
                                            let l62 = i32::from(*((ptr1 + 16) as *const u8));
                                            match l62 { 0 => None , 1 => { let e = { let l63 = * ((ptr1 + 20) as * const i32) ; let l64 = * ((ptr1 + 24) as * const i32) ; let len65 = l64 as usize ; let bytes65 = Vec :: from_raw_parts (l63 as * mut _ , len65 , len65) ; plugin :: wit_bindgen :: rt :: string_lift (bytes65) } ; Some (e) } _ => plugin :: wit_bindgen :: rt :: invalid_enum_discriminant () , }
                                        };
                                        V66::InternalError(e66)
                                    }
                                };
                                v66
                            };
                            Err(e)
                        }
                        _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                    }
                }
            }
        }
    }
    pub mod io {
        #[allow(clippy::all)]
        pub mod poll {
            #[used]
            #[doc(hidden)]
            #[cfg(target_arch = "wasm32")]
            static __FORCE_SECTION_REF: fn() = super::super::super::__link_section;
            /// `pollable` represents a single I/O event which may be ready, or not.
            #[repr(transparent)]
            pub struct Pollable {
                handle: plugin::wit_bindgen::rt::Resource<Pollable>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for Pollable {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "Pollable",
                        "handle",
                        &&self.handle,
                    )
                }
            }
            impl Pollable {
                #[doc(hidden)]
                pub unsafe fn from_handle(handle: u32) -> Self {
                    Self {
                        handle: plugin::wit_bindgen::rt::Resource::from_handle(handle),
                    }
                }
                #[doc(hidden)]
                pub fn into_handle(self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::into_handle(self.handle)
                }
                #[doc(hidden)]
                pub fn handle(&self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::handle(&self.handle)
                }
            }
            unsafe impl plugin::wit_bindgen::rt::WasmResource for Pollable {
                #[inline]
                unsafe fn drop(_handle: u32) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        #[link(wasm_import_module = "wasi:io/poll@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[resource-drop]pollable"]
                            fn drop(_: u32);
                        }
                        drop(_handle);
                    }
                }
            }
            impl Pollable {
                #[allow(unused_unsafe, clippy::all)]
                /// Return the readiness of a pollable. This function never blocks.
                ///
                /// Returns `true` when the pollable is ready, and `false` otherwise.
                pub fn ready(&self) -> bool {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/poll@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]pollable.ready"]
                            fn wit_import(_: i32) -> i32;
                        }
                        let ret = wit_import((self).handle() as i32);
                        plugin::wit_bindgen::rt::bool_lift(ret as u8)
                    }
                }
            }
            impl Pollable {
                #[allow(unused_unsafe, clippy::all)]
                /// `block` returns immediately if the pollable is ready, and otherwise
                /// blocks until ready.
                ///
                /// This function is equivalent to calling `poll.poll` on a list
                /// containing only this pollable.
                pub fn block(&self) {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/poll@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]pollable.block"]
                            fn wit_import(_: i32);
                        }
                        wit_import((self).handle() as i32);
                    }
                }
            }
            #[allow(unused_unsafe, clippy::all)]
            /// Poll for completion on a set of pollables.
            ///
            /// This function takes a list of pollables, which identify I/O sources of
            /// interest, and waits until one or more of the events is ready for I/O.
            ///
            /// The result `list<u32>` contains one or more indices of handles in the
            /// argument list that is ready for I/O.
            ///
            /// If the list contains more elements than can be indexed with a `u32`
            /// value, this function traps.
            ///
            /// A timeout can be implemented by adding a pollable from the
            /// wasi-clocks API to the list.
            ///
            /// This function does not return a `result`; polling in itself does not
            /// do any I/O so it doesn't fail. If any of the I/O sources identified by
            /// the pollables has an error, it is indicated by marking the source as
            /// being reaedy for I/O.
            pub fn poll(in_: &[&Pollable]) -> plugin::wit_bindgen::rt::vec::Vec<u32> {
                #[allow(unused_imports)]
                use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                unsafe {
                    #[repr(align(4))]
                    struct RetArea([u8; 8]);
                    let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                    let vec0 = in_;
                    let len0 = vec0.len() as i32;
                    let layout0 = alloc::Layout::from_size_align_unchecked(vec0.len() * 4, 4);
                    let result0 = if layout0.size() != 0 {
                        let ptr = alloc::alloc(layout0);
                        if ptr.is_null() {
                            alloc::handle_alloc_error(layout0);
                        }
                        ptr
                    } else {
                        {
                            ::core::ptr::null_mut()
                        }
                    };
                    for (i, e) in vec0.into_iter().enumerate() {
                        let base = result0 as i32 + (i as i32) * 4;
                        {
                            *((base + 0) as *mut i32) = (e).handle() as i32;
                        }
                    }
                    let ptr1 = ret_area.as_mut_ptr() as i32;
                    #[cfg(target_arch = "wasm32")]
                    #[link(wasm_import_module = "wasi:io/poll@0.2.0-rc-2023-11-10")]
                    extern "C" {
                        #[link_name = "poll"]
                        fn wit_import(_: i32, _: i32, _: i32);
                    }
                    wit_import(result0 as i32, len0, ptr1);
                    let l2 = *((ptr1 + 0) as *const i32);
                    let l3 = *((ptr1 + 4) as *const i32);
                    let len4 = l3 as usize;
                    if layout0.size() != 0 {
                        alloc::dealloc(result0, layout0);
                    }
                    Vec::from_raw_parts(l2 as *mut _, len4, len4)
                }
            }
        }
        #[allow(clippy::all)]
        pub mod error {
            #[used]
            #[doc(hidden)]
            #[cfg(target_arch = "wasm32")]
            static __FORCE_SECTION_REF: fn() = super::super::super::__link_section;
            /// A resource which represents some error information.
            ///
            /// The only method provided by this resource is `to-debug-string`,
            /// which provides some human-readable information about the error.
            ///
            /// In the `wasi:io` package, this resource is returned through the
            /// `wasi:io/streams/stream-error` type.
            ///
            /// To provide more specific error information, other interfaces may
            /// provide functions to further "downcast" this error into more specific
            /// error information. For example, `error`s returned in streams derived
            /// from filesystem types to be described using the filesystem's own
            /// error-code type, using the function
            /// `wasi:filesystem/types/filesystem-error-code`, which takes a parameter
            /// `borrow<error>` and returns
            /// `option<wasi:filesystem/types/error-code>`.
            ///
            /// The set of functions which can "downcast" an `error` into a more
            /// concrete type is open.
            #[repr(transparent)]
            pub struct Error {
                handle: plugin::wit_bindgen::rt::Resource<Error>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for Error {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "Error",
                        "handle",
                        &&self.handle,
                    )
                }
            }
            impl Error {
                #[doc(hidden)]
                pub unsafe fn from_handle(handle: u32) -> Self {
                    Self {
                        handle: plugin::wit_bindgen::rt::Resource::from_handle(handle),
                    }
                }
                #[doc(hidden)]
                pub fn into_handle(self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::into_handle(self.handle)
                }
                #[doc(hidden)]
                pub fn handle(&self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::handle(&self.handle)
                }
            }
            unsafe impl plugin::wit_bindgen::rt::WasmResource for Error {
                #[inline]
                unsafe fn drop(_handle: u32) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        #[link(wasm_import_module = "wasi:io/error@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[resource-drop]error"]
                            fn drop(_: u32);
                        }
                        drop(_handle);
                    }
                }
            }
            impl Error {
                #[allow(unused_unsafe, clippy::all)]
                /// Returns a string that is suitable to assist humans in debugging
                /// this error.
                ///
                /// WARNING: The returned string should not be consumed mechanically!
                /// It may change across platforms, hosts, or other implementation
                /// details. Parsing this string is a major platform-compatibility
                /// hazard.
                pub fn to_debug_string(&self) -> plugin::wit_bindgen::rt::string::String {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 8]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/error@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]error.to-debug-string"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = *((ptr0 + 0) as *const i32);
                        let l2 = *((ptr0 + 4) as *const i32);
                        let len3 = l2 as usize;
                        let bytes3 = Vec::from_raw_parts(l1 as *mut _, len3, len3);
                        plugin::wit_bindgen::rt::string_lift(bytes3)
                    }
                }
            }
        }
        #[allow(clippy::all)]
        pub mod streams {
            #[used]
            #[doc(hidden)]
            #[cfg(target_arch = "wasm32")]
            static __FORCE_SECTION_REF: fn() = super::super::super::__link_section;
            pub type Error = super::super::super::wasi::io::error::Error;
            pub type Pollable = super::super::super::wasi::io::poll::Pollable;
            /// An error for input-stream and output-stream operations.
            pub enum StreamError {
                /// The last operation (a write or flush) failed before completion.
                ///
                /// More information is available in the `error` payload.
                LastOperationFailed(Error),
                /// The stream is closed: no more input will be accepted by the
                /// stream. A closed output-stream will return this error on all
                /// future operations.
                Closed,
            }
            impl ::core::fmt::Debug for StreamError {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    match self {
                        StreamError::LastOperationFailed(e) => f
                            .debug_tuple("StreamError::LastOperationFailed")
                            .field(e)
                            .finish(),
                        StreamError::Closed => f.debug_tuple("StreamError::Closed").finish(),
                    }
                }
            }
            impl ::core::fmt::Display for StreamError {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    f.write_fmt(format_args!("{0:?}", self))
                }
            }
            impl std::error::Error for StreamError {}
            /// An input bytestream.
            ///
            /// `input-stream`s are *non-blocking* to the extent practical on underlying
            /// platforms. I/O operations always return promptly; if fewer bytes are
            /// promptly available than requested, they return the number of bytes promptly
            /// available, which could even be zero. To wait for data to be available,
            /// use the `subscribe` function to obtain a `pollable` which can be polled
            /// for using `wasi:io/poll`.
            #[repr(transparent)]
            pub struct InputStream {
                handle: plugin::wit_bindgen::rt::Resource<InputStream>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for InputStream {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "InputStream",
                        "handle",
                        &&self.handle,
                    )
                }
            }
            impl InputStream {
                #[doc(hidden)]
                pub unsafe fn from_handle(handle: u32) -> Self {
                    Self {
                        handle: plugin::wit_bindgen::rt::Resource::from_handle(handle),
                    }
                }
                #[doc(hidden)]
                pub fn into_handle(self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::into_handle(self.handle)
                }
                #[doc(hidden)]
                pub fn handle(&self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::handle(&self.handle)
                }
            }
            unsafe impl plugin::wit_bindgen::rt::WasmResource for InputStream {
                #[inline]
                unsafe fn drop(_handle: u32) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[resource-drop]input-stream"]
                            fn drop(_: u32);
                        }
                        drop(_handle);
                    }
                }
            }
            /// An output bytestream.
            ///
            /// `output-stream`s are *non-blocking* to the extent practical on
            /// underlying platforms. Except where specified otherwise, I/O operations also
            /// always return promptly, after the number of bytes that can be written
            /// promptly, which could even be zero. To wait for the stream to be ready to
            /// accept data, the `subscribe` function to obtain a `pollable` which can be
            /// polled for using `wasi:io/poll`.
            #[repr(transparent)]
            pub struct OutputStream {
                handle: plugin::wit_bindgen::rt::Resource<OutputStream>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for OutputStream {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "OutputStream",
                        "handle",
                        &&self.handle,
                    )
                }
            }
            impl OutputStream {
                #[doc(hidden)]
                pub unsafe fn from_handle(handle: u32) -> Self {
                    Self {
                        handle: plugin::wit_bindgen::rt::Resource::from_handle(handle),
                    }
                }
                #[doc(hidden)]
                pub fn into_handle(self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::into_handle(self.handle)
                }
                #[doc(hidden)]
                pub fn handle(&self) -> u32 {
                    plugin::wit_bindgen::rt::Resource::handle(&self.handle)
                }
            }
            unsafe impl plugin::wit_bindgen::rt::WasmResource for OutputStream {
                #[inline]
                unsafe fn drop(_handle: u32) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[resource-drop]output-stream"]
                            fn drop(_: u32);
                        }
                        drop(_handle);
                    }
                }
            }
            impl InputStream {
                #[allow(unused_unsafe, clippy::all)]
                /// Perform a non-blocking read from the stream.
                ///
                /// This function returns a list of bytes containing the read data,
                /// when successful. The returned list will contain up to `len` bytes;
                /// it may return fewer than requested, but not more. The list is
                /// empty when no bytes are available for reading at this time. The
                /// pollable given by `subscribe` will be ready when more bytes are
                /// available.
                ///
                /// This function fails with a `stream-error` when the operation
                /// encounters an error, giving `last-operation-failed`, or when the
                /// stream is closed, giving `closed`.
                ///
                /// When the caller gives a `len` of 0, it represents a request to
                /// read 0 bytes. If the stream is still open, this call should
                /// succeed and return an empty list, or otherwise fail with `closed`.
                ///
                /// The `len` parameter is a `u64`, which could represent a list of u8 which
                /// is not possible to allocate in wasm32, or not desirable to allocate as
                /// as a return value by the callee. The callee may return a list of bytes
                /// less than `len` in size while more bytes are available for reading.
                pub fn read(
                    &self,
                    len: u64,
                ) -> Result<plugin::wit_bindgen::rt::vec::Vec<u8>, StreamError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 12]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]input-stream.read"]
                            fn wit_import(_: i32, _: i64, _: i32);
                        }
                        wit_import(
                            (self).handle() as i32,
                            plugin::wit_bindgen::rt::as_i64(len),
                            ptr0,
                        );
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = {
                                    let l2 = *((ptr0 + 4) as *const i32);
                                    let l3 = *((ptr0 + 8) as *const i32);
                                    let len4 = l3 as usize;
                                    Vec::from_raw_parts(l2 as *mut _, len4, len4)
                                };
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l5 = i32::from(*((ptr0 + 4) as *const u8));
                                    let v7 = match l5 {
                                        0 => {
                                            let e7 = {
                                                let l6 = *((ptr0 + 8) as *const i32);
                                                super :: super :: super :: wasi :: io :: error :: Error :: from_handle (l6 as u32)
                                            };
                                            StreamError::LastOperationFailed(e7)
                                        }
                                        n => {
                                            if true {
                                                match (&n, &1) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            StreamError::Closed
                                        }
                                    };
                                    v7
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl InputStream {
                #[allow(unused_unsafe, clippy::all)]
                /// Read bytes from a stream, after blocking until at least one byte can
                /// be read. Except for blocking, behavior is identical to `read`.
                pub fn blocking_read(
                    &self,
                    len: u64,
                ) -> Result<plugin::wit_bindgen::rt::vec::Vec<u8>, StreamError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 12]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]input-stream.blocking-read"]
                            fn wit_import(_: i32, _: i64, _: i32);
                        }
                        wit_import(
                            (self).handle() as i32,
                            plugin::wit_bindgen::rt::as_i64(len),
                            ptr0,
                        );
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = {
                                    let l2 = *((ptr0 + 4) as *const i32);
                                    let l3 = *((ptr0 + 8) as *const i32);
                                    let len4 = l3 as usize;
                                    Vec::from_raw_parts(l2 as *mut _, len4, len4)
                                };
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l5 = i32::from(*((ptr0 + 4) as *const u8));
                                    let v7 = match l5 {
                                        0 => {
                                            let e7 = {
                                                let l6 = *((ptr0 + 8) as *const i32);
                                                super :: super :: super :: wasi :: io :: error :: Error :: from_handle (l6 as u32)
                                            };
                                            StreamError::LastOperationFailed(e7)
                                        }
                                        n => {
                                            if true {
                                                match (&n, &1) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            StreamError::Closed
                                        }
                                    };
                                    v7
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl InputStream {
                #[allow(unused_unsafe, clippy::all)]
                /// Skip bytes from a stream. Returns number of bytes skipped.
                ///
                /// Behaves identical to `read`, except instead of returning a list
                /// of bytes, returns the number of bytes consumed from the stream.
                pub fn skip(&self, len: u64) -> Result<u64, StreamError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(8))]
                        struct RetArea([u8; 16]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]input-stream.skip"]
                            fn wit_import(_: i32, _: i64, _: i32);
                        }
                        wit_import(
                            (self).handle() as i32,
                            plugin::wit_bindgen::rt::as_i64(len),
                            ptr0,
                        );
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = {
                                    let l2 = *((ptr0 + 8) as *const i64);
                                    l2 as u64
                                };
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l3 = i32::from(*((ptr0 + 8) as *const u8));
                                    let v5 = match l3 {
                                        0 => {
                                            let e5 = {
                                                let l4 = *((ptr0 + 12) as *const i32);
                                                super :: super :: super :: wasi :: io :: error :: Error :: from_handle (l4 as u32)
                                            };
                                            StreamError::LastOperationFailed(e5)
                                        }
                                        n => {
                                            if true {
                                                match (&n, &1) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            StreamError::Closed
                                        }
                                    };
                                    v5
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl InputStream {
                #[allow(unused_unsafe, clippy::all)]
                /// Skip bytes from a stream, after blocking until at least one byte
                /// can be skipped. Except for blocking behavior, identical to `skip`.
                pub fn blocking_skip(&self, len: u64) -> Result<u64, StreamError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(8))]
                        struct RetArea([u8; 16]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]input-stream.blocking-skip"]
                            fn wit_import(_: i32, _: i64, _: i32);
                        }
                        wit_import(
                            (self).handle() as i32,
                            plugin::wit_bindgen::rt::as_i64(len),
                            ptr0,
                        );
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = {
                                    let l2 = *((ptr0 + 8) as *const i64);
                                    l2 as u64
                                };
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l3 = i32::from(*((ptr0 + 8) as *const u8));
                                    let v5 = match l3 {
                                        0 => {
                                            let e5 = {
                                                let l4 = *((ptr0 + 12) as *const i32);
                                                super :: super :: super :: wasi :: io :: error :: Error :: from_handle (l4 as u32)
                                            };
                                            StreamError::LastOperationFailed(e5)
                                        }
                                        n => {
                                            if true {
                                                match (&n, &1) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            StreamError::Closed
                                        }
                                    };
                                    v5
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl InputStream {
                #[allow(unused_unsafe, clippy::all)]
                /// Create a `pollable` which will resolve once either the specified stream
                /// has bytes available to read or the other end of the stream has been
                /// closed.
                /// The created `pollable` is a child resource of the `input-stream`.
                /// Implementations may trap if the `input-stream` is dropped before
                /// all derived `pollable`s created with this function are dropped.
                pub fn subscribe(&self) -> Pollable {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]input-stream.subscribe"]
                            fn wit_import(_: i32) -> i32;
                        }
                        let ret = wit_import((self).handle() as i32);
                        super::super::super::wasi::io::poll::Pollable::from_handle(ret as u32)
                    }
                }
            }
            impl OutputStream {
                #[allow(unused_unsafe, clippy::all)]
                /// Check readiness for writing. This function never blocks.
                ///
                /// Returns the number of bytes permitted for the next call to `write`,
                /// or an error. Calling `write` with more bytes than this function has
                /// permitted will trap.
                ///
                /// When this function returns 0 bytes, the `subscribe` pollable will
                /// become ready when this function will report at least 1 byte, or an
                /// error.
                pub fn check_write(&self) -> Result<u64, StreamError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(8))]
                        struct RetArea([u8; 16]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]output-stream.check-write"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = {
                                    let l2 = *((ptr0 + 8) as *const i64);
                                    l2 as u64
                                };
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l3 = i32::from(*((ptr0 + 8) as *const u8));
                                    let v5 = match l3 {
                                        0 => {
                                            let e5 = {
                                                let l4 = *((ptr0 + 12) as *const i32);
                                                super :: super :: super :: wasi :: io :: error :: Error :: from_handle (l4 as u32)
                                            };
                                            StreamError::LastOperationFailed(e5)
                                        }
                                        n => {
                                            if true {
                                                match (&n, &1) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            StreamError::Closed
                                        }
                                    };
                                    v5
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutputStream {
                #[allow(unused_unsafe, clippy::all)]
                /// Perform a write. This function never blocks.
                ///
                /// Precondition: check-write gave permit of Ok(n) and contents has a
                /// length of less than or equal to n. Otherwise, this function will trap.
                ///
                /// returns Err(closed) without writing if the stream has closed since
                /// the last call to check-write provided a permit.
                pub fn write(&self, contents: &[u8]) -> Result<(), StreamError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 12]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let vec0 = contents;
                        let ptr0 = vec0.as_ptr() as i32;
                        let len0 = vec0.len() as i32;
                        let ptr1 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]output-stream.write"]
                            fn wit_import(_: i32, _: i32, _: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0, len0, ptr1);
                        let l2 = i32::from(*((ptr1 + 0) as *const u8));
                        match l2 {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l3 = i32::from(*((ptr1 + 4) as *const u8));
                                    let v5 = match l3 {
                                        0 => {
                                            let e5 = {
                                                let l4 = *((ptr1 + 8) as *const i32);
                                                super :: super :: super :: wasi :: io :: error :: Error :: from_handle (l4 as u32)
                                            };
                                            StreamError::LastOperationFailed(e5)
                                        }
                                        n => {
                                            if true {
                                                match (&n, &1) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            StreamError::Closed
                                        }
                                    };
                                    v5
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutputStream {
                #[allow(unused_unsafe, clippy::all)]
                /// Perform a write of up to 4096 bytes, and then flush the stream. Block
                /// until all of these operations are complete, or an error occurs.
                ///
                /// This is a convenience wrapper around the use of `check-write`,
                /// `subscribe`, `write`, and `flush`, and is implemented with the
                /// following pseudo-code:
                ///
                /// ```text
                /// let pollable = this.subscribe();
                /// while !contents.is_empty() {
                /// // Wait for the stream to become writable
                /// pollable.block();
                /// let Ok(n) = this.check-write(); // eliding error handling
                /// let len = min(n, contents.len());
                /// let (chunk, rest) = contents.split_at(len);
                /// this.write(chunk  );            // eliding error handling
                /// contents = rest;
                /// }
                /// this.flush();
                /// // Wait for completion of `flush`
                /// pollable.block();
                /// // Check for any errors that arose during `flush`
                /// let _ = this.check-write();         // eliding error handling
                /// ```
                pub fn blocking_write_and_flush(&self, contents: &[u8]) -> Result<(), StreamError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 12]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let vec0 = contents;
                        let ptr0 = vec0.as_ptr() as i32;
                        let len0 = vec0.len() as i32;
                        let ptr1 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]output-stream.blocking-write-and-flush"]
                            fn wit_import(_: i32, _: i32, _: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0, len0, ptr1);
                        let l2 = i32::from(*((ptr1 + 0) as *const u8));
                        match l2 {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l3 = i32::from(*((ptr1 + 4) as *const u8));
                                    let v5 = match l3 {
                                        0 => {
                                            let e5 = {
                                                let l4 = *((ptr1 + 8) as *const i32);
                                                super :: super :: super :: wasi :: io :: error :: Error :: from_handle (l4 as u32)
                                            };
                                            StreamError::LastOperationFailed(e5)
                                        }
                                        n => {
                                            if true {
                                                match (&n, &1) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            StreamError::Closed
                                        }
                                    };
                                    v5
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutputStream {
                #[allow(unused_unsafe, clippy::all)]
                /// Request to flush buffered output. This function never blocks.
                ///
                /// This tells the output-stream that the caller intends any buffered
                /// output to be flushed. the output which is expected to be flushed
                /// is all that has been passed to `write` prior to this call.
                ///
                /// Upon calling this function, the `output-stream` will not accept any
                /// writes (`check-write` will return `ok(0)`) until the flush has
                /// completed. The `subscribe` pollable will become ready when the
                /// flush has completed and the stream can accept more writes.
                pub fn flush(&self) -> Result<(), StreamError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 12]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]output-stream.flush"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l2 = i32::from(*((ptr0 + 4) as *const u8));
                                    let v4 = match l2 {
                                        0 => {
                                            let e4 = {
                                                let l3 = *((ptr0 + 8) as *const i32);
                                                super :: super :: super :: wasi :: io :: error :: Error :: from_handle (l3 as u32)
                                            };
                                            StreamError::LastOperationFailed(e4)
                                        }
                                        n => {
                                            if true {
                                                match (&n, &1) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            StreamError::Closed
                                        }
                                    };
                                    v4
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutputStream {
                #[allow(unused_unsafe, clippy::all)]
                /// Request to flush buffered output, and block until flush completes
                /// and stream is ready for writing again.
                pub fn blocking_flush(&self) -> Result<(), StreamError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 12]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]output-stream.blocking-flush"]
                            fn wit_import(_: i32, _: i32);
                        }
                        wit_import((self).handle() as i32, ptr0);
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l2 = i32::from(*((ptr0 + 4) as *const u8));
                                    let v4 = match l2 {
                                        0 => {
                                            let e4 = {
                                                let l3 = *((ptr0 + 8) as *const i32);
                                                super :: super :: super :: wasi :: io :: error :: Error :: from_handle (l3 as u32)
                                            };
                                            StreamError::LastOperationFailed(e4)
                                        }
                                        n => {
                                            if true {
                                                match (&n, &1) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            StreamError::Closed
                                        }
                                    };
                                    v4
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutputStream {
                #[allow(unused_unsafe, clippy::all)]
                /// Create a `pollable` which will resolve once the output-stream
                /// is ready for more writing, or an error has occured. When this
                /// pollable is ready, `check-write` will return `ok(n)` with n>0, or an
                /// error.
                ///
                /// If the stream is closed, this pollable is always ready immediately.
                ///
                /// The created `pollable` is a child resource of the `output-stream`.
                /// Implementations may trap if the `output-stream` is dropped before
                /// all derived `pollable`s created with this function are dropped.
                pub fn subscribe(&self) -> Pollable {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]output-stream.subscribe"]
                            fn wit_import(_: i32) -> i32;
                        }
                        let ret = wit_import((self).handle() as i32);
                        super::super::super::wasi::io::poll::Pollable::from_handle(ret as u32)
                    }
                }
            }
            impl OutputStream {
                #[allow(unused_unsafe, clippy::all)]
                /// Write zeroes to a stream.
                ///
                /// This should be used precisely like `write` with the exact same
                /// preconditions (must use check-write first), but instead of
                /// passing a list of bytes, you simply pass the number of zero-bytes
                /// that should be written.
                pub fn write_zeroes(&self, len: u64) -> Result<(), StreamError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 12]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]output-stream.write-zeroes"]
                            fn wit_import(_: i32, _: i64, _: i32);
                        }
                        wit_import(
                            (self).handle() as i32,
                            plugin::wit_bindgen::rt::as_i64(len),
                            ptr0,
                        );
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l2 = i32::from(*((ptr0 + 4) as *const u8));
                                    let v4 = match l2 {
                                        0 => {
                                            let e4 = {
                                                let l3 = *((ptr0 + 8) as *const i32);
                                                super :: super :: super :: wasi :: io :: error :: Error :: from_handle (l3 as u32)
                                            };
                                            StreamError::LastOperationFailed(e4)
                                        }
                                        n => {
                                            if true {
                                                match (&n, &1) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            StreamError::Closed
                                        }
                                    };
                                    v4
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutputStream {
                #[allow(unused_unsafe, clippy::all)]
                /// Perform a write of up to 4096 zeroes, and then flush the stream.
                /// Block until all of these operations are complete, or an error
                /// occurs.
                ///
                /// This is a convenience wrapper around the use of `check-write`,
                /// `subscribe`, `write-zeroes`, and `flush`, and is implemented with
                /// the following pseudo-code:
                ///
                /// ```text
                /// let pollable = this.subscribe();
                /// while num_zeroes != 0 {
                /// // Wait for the stream to become writable
                /// pollable.block();
                /// let Ok(n) = this.check-write(); // eliding error handling
                /// let len = min(n, num_zeroes);
                /// this.write-zeroes(len);         // eliding error handling
                /// num_zeroes -= len;
                /// }
                /// this.flush();
                /// // Wait for completion of `flush`
                /// pollable.block();
                /// // Check for any errors that arose during `flush`
                /// let _ = this.check-write();         // eliding error handling
                /// ```
                pub fn blocking_write_zeroes_and_flush(&self, len: u64) -> Result<(), StreamError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(4))]
                        struct RetArea([u8; 12]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]output-stream.blocking-write-zeroes-and-flush"]
                            fn wit_import(_: i32, _: i64, _: i32);
                        }
                        wit_import(
                            (self).handle() as i32,
                            plugin::wit_bindgen::rt::as_i64(len),
                            ptr0,
                        );
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = ();
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l2 = i32::from(*((ptr0 + 4) as *const u8));
                                    let v4 = match l2 {
                                        0 => {
                                            let e4 = {
                                                let l3 = *((ptr0 + 8) as *const i32);
                                                super :: super :: super :: wasi :: io :: error :: Error :: from_handle (l3 as u32)
                                            };
                                            StreamError::LastOperationFailed(e4)
                                        }
                                        n => {
                                            if true {
                                                match (&n, &1) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            StreamError::Closed
                                        }
                                    };
                                    v4
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutputStream {
                #[allow(unused_unsafe, clippy::all)]
                /// Read from one stream and write to another.
                ///
                /// The behavior of splice is equivelant to:
                /// 1. calling `check-write` on the `output-stream`
                /// 2. calling `read` on the `input-stream` with the smaller of the
                /// `check-write` permitted length and the `len` provided to `splice`
                /// 3. calling `write` on the `output-stream` with that read data.
                ///
                /// Any error reported by the call to `check-write`, `read`, or
                /// `write` ends the splice and reports that error.
                ///
                /// This function returns the number of bytes transferred; it may be less
                /// than `len`.
                pub fn splice(&self, src: &InputStream, len: u64) -> Result<u64, StreamError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(8))]
                        struct RetArea([u8; 16]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]output-stream.splice"]
                            fn wit_import(_: i32, _: i32, _: i64, _: i32);
                        }
                        wit_import(
                            (self).handle() as i32,
                            (src).handle() as i32,
                            plugin::wit_bindgen::rt::as_i64(len),
                            ptr0,
                        );
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = {
                                    let l2 = *((ptr0 + 8) as *const i64);
                                    l2 as u64
                                };
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l3 = i32::from(*((ptr0 + 8) as *const u8));
                                    let v5 = match l3 {
                                        0 => {
                                            let e5 = {
                                                let l4 = *((ptr0 + 12) as *const i32);
                                                super :: super :: super :: wasi :: io :: error :: Error :: from_handle (l4 as u32)
                                            };
                                            StreamError::LastOperationFailed(e5)
                                        }
                                        n => {
                                            if true {
                                                match (&n, &1) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            StreamError::Closed
                                        }
                                    };
                                    v5
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
            impl OutputStream {
                #[allow(unused_unsafe, clippy::all)]
                /// Read from one stream and write to another, with blocking.
                ///
                /// This is similar to `splice`, except that it blocks until the
                /// `output-stream` is ready for writing, and the `input-stream`
                /// is ready for reading, before performing the `splice`.
                pub fn blocking_splice(
                    &self,
                    src: &InputStream,
                    len: u64,
                ) -> Result<u64, StreamError> {
                    #[allow(unused_imports)]
                    use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                    unsafe {
                        #[repr(align(8))]
                        struct RetArea([u8; 16]);
                        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                        let ptr0 = ret_area.as_mut_ptr() as i32;
                        #[cfg(target_arch = "wasm32")]
                        #[link(wasm_import_module = "wasi:io/streams@0.2.0-rc-2023-11-10")]
                        extern "C" {
                            #[link_name = "[method]output-stream.blocking-splice"]
                            fn wit_import(_: i32, _: i32, _: i64, _: i32);
                        }
                        wit_import(
                            (self).handle() as i32,
                            (src).handle() as i32,
                            plugin::wit_bindgen::rt::as_i64(len),
                            ptr0,
                        );
                        let l1 = i32::from(*((ptr0 + 0) as *const u8));
                        match l1 {
                            0 => {
                                let e = {
                                    let l2 = *((ptr0 + 8) as *const i64);
                                    l2 as u64
                                };
                                Ok(e)
                            }
                            1 => {
                                let e = {
                                    let l3 = i32::from(*((ptr0 + 8) as *const u8));
                                    let v5 = match l3 {
                                        0 => {
                                            let e5 = {
                                                let l4 = *((ptr0 + 12) as *const i32);
                                                super :: super :: super :: wasi :: io :: error :: Error :: from_handle (l4 as u32)
                                            };
                                            StreamError::LastOperationFailed(e5)
                                        }
                                        n => {
                                            if true {
                                                match (&n, &1) {
                                                    (left_val, right_val) => {
                                                        if !(*left_val == *right_val) {
                                                            let kind =
                                                                ::core::panicking::AssertKind::Eq;
                                                            ::core::panicking::assert_failed(
                                                                kind,
                                                                &*left_val,
                                                                &*right_val,
                                                                ::core::option::Option::Some(
                                                                    format_args!(
                                                                        "invalid enum discriminant"
                                                                    ),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                };
                                            };
                                            StreamError::Closed
                                        }
                                    };
                                    v5
                                };
                                Err(e)
                            }
                            _ => plugin::wit_bindgen::rt::invalid_enum_discriminant(),
                        }
                    }
                }
            }
        }
    }
}
pub mod exports {
    pub mod litehouse {
        pub mod plugin {
            #[allow(clippy::all)]
            pub mod plugin {
                #[used]
                #[doc(hidden)]
                #[cfg(target_arch = "wasm32")]
                static __FORCE_SECTION_REF: fn() = super::super::super::super::__link_section;
                pub enum Update {
                    Time(u64),
                    Temperature(f64),
                    WindSpeed(f64),
                }
                #[automatically_derived]
                impl ::core::clone::Clone for Update {
                    #[inline]
                    fn clone(&self) -> Update {
                        let _: ::core::clone::AssertParamIsClone<u64>;
                        let _: ::core::clone::AssertParamIsClone<f64>;
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for Update {}
                impl ::core::fmt::Debug for Update {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        match self {
                            Update::Time(e) => f.debug_tuple("Update::Time").field(e).finish(),
                            Update::Temperature(e) => {
                                f.debug_tuple("Update::Temperature").field(e).finish()
                            }
                            Update::WindSpeed(e) => {
                                f.debug_tuple("Update::WindSpeed").field(e).finish()
                            }
                        }
                    }
                }
                #[repr(C)]
                pub struct Event {
                    pub id: u64,
                    pub timestamp: u64,
                    pub inner: Update,
                }
                #[automatically_derived]
                impl ::core::clone::Clone for Event {
                    #[inline]
                    fn clone(&self) -> Event {
                        let _: ::core::clone::AssertParamIsClone<u64>;
                        let _: ::core::clone::AssertParamIsClone<Update>;
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for Event {}
                impl ::core::fmt::Debug for Event {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        f.debug_struct("Event")
                            .field("id", &self.id)
                            .field("timestamp", &self.timestamp)
                            .field("inner", &self.inner)
                            .finish()
                    }
                }
                pub enum TimeUnit {
                    Second,
                    Minute,
                    Hour,
                    Day,
                    Week,
                    Month,
                    Year,
                }
                #[automatically_derived]
                impl ::core::clone::Clone for TimeUnit {
                    #[inline]
                    fn clone(&self) -> TimeUnit {
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for TimeUnit {}
                impl ::core::fmt::Debug for TimeUnit {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        match self {
                            TimeUnit::Second => f.debug_tuple("TimeUnit::Second").finish(),
                            TimeUnit::Minute => f.debug_tuple("TimeUnit::Minute").finish(),
                            TimeUnit::Hour => f.debug_tuple("TimeUnit::Hour").finish(),
                            TimeUnit::Day => f.debug_tuple("TimeUnit::Day").finish(),
                            TimeUnit::Week => f.debug_tuple("TimeUnit::Week").finish(),
                            TimeUnit::Month => f.debug_tuple("TimeUnit::Month").finish(),
                            TimeUnit::Year => f.debug_tuple("TimeUnit::Year").finish(),
                        }
                    }
                }
                #[repr(C)]
                pub struct Every {
                    pub amount: u64,
                    pub unit: TimeUnit,
                }
                #[automatically_derived]
                impl ::core::clone::Clone for Every {
                    #[inline]
                    fn clone(&self) -> Every {
                        let _: ::core::clone::AssertParamIsClone<u64>;
                        let _: ::core::clone::AssertParamIsClone<TimeUnit>;
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for Every {}
                impl ::core::fmt::Debug for Every {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        f.debug_struct("Every")
                            .field("amount", &self.amount)
                            .field("unit", &self.unit)
                            .finish()
                    }
                }
                pub enum TimeSubscription {
                    Every(Every),
                    At(u64),
                }
                #[automatically_derived]
                impl ::core::clone::Clone for TimeSubscription {
                    #[inline]
                    fn clone(&self) -> TimeSubscription {
                        let _: ::core::clone::AssertParamIsClone<Every>;
                        let _: ::core::clone::AssertParamIsClone<u64>;
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for TimeSubscription {}
                impl ::core::fmt::Debug for TimeSubscription {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        match self {
                            TimeSubscription::Every(e) => {
                                f.debug_tuple("TimeSubscription::Every").field(e).finish()
                            }
                            TimeSubscription::At(e) => {
                                f.debug_tuple("TimeSubscription::At").field(e).finish()
                            }
                        }
                    }
                }
                pub enum Subscription {
                    Time(TimeSubscription),
                }
                #[automatically_derived]
                impl ::core::clone::Clone for Subscription {
                    #[inline]
                    fn clone(&self) -> Subscription {
                        let _: ::core::clone::AssertParamIsClone<TimeSubscription>;
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for Subscription {}
                impl ::core::fmt::Debug for Subscription {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        match self {
                            Subscription::Time(e) => {
                                f.debug_tuple("Subscription::Time").field(e).finish()
                            }
                        }
                    }
                }
                pub use super::super::super::super::TasmotaPlugin as Runner;
                const _: () = {
                    #[doc(hidden)]
                    #[export_name = "litehouse:plugin/plugin#[dtor]runner"]
                    #[allow(non_snake_case)]
                    unsafe extern "C" fn dtor(rep: usize) {
                        plugin::wit_bindgen::rt::Resource::<Runner>::dtor(rep)
                    }
                };
                unsafe impl plugin::wit_bindgen::rt::RustResource for Runner {
                    unsafe fn new(_rep: usize) -> u32 {
                        #[cfg(target_arch = "wasm32")]
                        {
                            #[link(wasm_import_module = "[export]litehouse:plugin/plugin")]
                            extern "C" {
                                #[link_name = "[resource-new]runner"]
                                fn new(_: usize) -> u32;
                            }
                            new(_rep)
                        }
                    }
                    unsafe fn rep(_handle: u32) -> usize {
                        #[cfg(target_arch = "wasm32")]
                        {
                            #[link(wasm_import_module = "[export]litehouse:plugin/plugin")]
                            extern "C" {
                                #[link_name = "[resource-rep]runner"]
                                fn rep(_: u32) -> usize;
                            }
                            rep(_handle)
                        }
                    }
                }
                pub type OwnRunner = plugin::wit_bindgen::rt::Resource<Runner>;
                unsafe impl plugin::wit_bindgen::rt::WasmResource for Runner {
                    #[inline]
                    unsafe fn drop(_handle: u32) {
                        #[cfg(target_arch = "wasm32")]
                        {
                            #[link(wasm_import_module = "[export]litehouse:plugin/plugin")]
                            extern "C" {
                                #[link_name = "[resource-drop]runner"]
                                fn drop(_: u32);
                            }
                            drop(_handle);
                        }
                    }
                }
                const _: () = {
                    #[doc(hidden)]
                    #[export_name = "litehouse:plugin/plugin#[constructor]runner"]
                    #[allow(non_snake_case)]
                    unsafe extern "C" fn __export_constructor_runner() -> i32 {
                        #[allow(unused_imports)]
                        use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                        #[cfg(target_arch = "wasm32")]
                        plugin::wit_bindgen::rt::run_ctors_once();
                        let result0 = OwnRunner::new(<_RunnerImpl as GuestRunner>::new());
                        plugin::wit_bindgen::rt::Resource::into_handle(result0) as i32
                    }
                };
                const _: () = {
                    #[doc(hidden)]
                    #[export_name = "litehouse:plugin/plugin#[method]runner.subscribe"]
                    #[allow(non_snake_case)]
                    unsafe extern "C" fn __export_method_runner_subscribe(arg0: i32) -> i32 {
                        #[allow(unused_imports)]
                        use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                        #[cfg(target_arch = "wasm32")]
                        plugin::wit_bindgen::rt::run_ctors_once();
                        let result0 = <_RunnerImpl as GuestRunner>::subscribe(
                            plugin::wit_bindgen::rt::Resource::<Runner>::lift_borrow(
                                arg0 as u32 as usize,
                            ),
                        );
                        let ptr1 = _RET_AREA.0.as_mut_ptr() as i32;
                        match result0 {
                            Ok(e) => {
                                *((ptr1 + 0) as *mut u8) = (0i32) as u8;
                                let vec3 = e;
                                let len3 = vec3.len() as i32;
                                let layout3 =
                                    alloc::Layout::from_size_align_unchecked(vec3.len() * 32, 8);
                                let result3 = if layout3.size() != 0 {
                                    let ptr = alloc::alloc(layout3);
                                    if ptr.is_null() {
                                        alloc::handle_alloc_error(layout3);
                                    }
                                    ptr
                                } else {
                                    {
                                        ::core::ptr::null_mut()
                                    }
                                };
                                for (i, e) in vec3.into_iter().enumerate() {
                                    let base = result3 as i32 + (i as i32) * 32;
                                    {
                                        match e {
                                            Subscription::Time(e) => {
                                                *((base + 0) as *mut u8) = (0i32) as u8;
                                                match e {
                                                    TimeSubscription::Every(e) => {
                                                        *((base + 8) as *mut u8) = (0i32) as u8;
                                                        let Every {
                                                            amount: amount2,
                                                            unit: unit2,
                                                        } = e;
                                                        *((base + 16) as *mut i64) =
                                                            plugin::wit_bindgen::rt::as_i64(
                                                                amount2,
                                                            );
                                                        match unit2 {
                                                            TimeUnit::Second => {
                                                                *((base + 24) as *mut u8) =
                                                                    (0i32) as u8;
                                                            }
                                                            TimeUnit::Minute => {
                                                                *((base + 24) as *mut u8) =
                                                                    (1i32) as u8;
                                                            }
                                                            TimeUnit::Hour => {
                                                                *((base + 24) as *mut u8) =
                                                                    (2i32) as u8;
                                                            }
                                                            TimeUnit::Day => {
                                                                *((base + 24) as *mut u8) =
                                                                    (3i32) as u8;
                                                            }
                                                            TimeUnit::Week => {
                                                                *((base + 24) as *mut u8) =
                                                                    (4i32) as u8;
                                                            }
                                                            TimeUnit::Month => {
                                                                *((base + 24) as *mut u8) =
                                                                    (5i32) as u8;
                                                            }
                                                            TimeUnit::Year => {
                                                                *((base + 24) as *mut u8) =
                                                                    (6i32) as u8;
                                                            }
                                                        }
                                                    }
                                                    TimeSubscription::At(e) => {
                                                        *((base + 8) as *mut u8) = (1i32) as u8;
                                                        *((base + 16) as *mut i64) =
                                                            plugin::wit_bindgen::rt::as_i64(e);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                *((ptr1 + 8) as *mut i32) = len3;
                                *((ptr1 + 4) as *mut i32) = result3 as i32;
                            }
                            Err(e) => {
                                *((ptr1 + 0) as *mut u8) = (1i32) as u8;
                                *((ptr1 + 4) as *mut i32) = plugin::wit_bindgen::rt::as_i32(e);
                            }
                        };
                        ptr1
                    }
                    const _: () = {
                        #[doc(hidden)]
                        #[export_name = "cabi_post_litehouse:plugin/plugin#[method]runner.subscribe"]
                        #[allow(non_snake_case)]
                        unsafe extern "C" fn __post_return_method_runner_subscribe(arg0: i32) {
                            let l0 = i32::from(*((arg0 + 0) as *const u8));
                            match l0 {
                                0 => {
                                    let l1 = *((arg0 + 4) as *const i32);
                                    let l2 = *((arg0 + 8) as *const i32);
                                    let base3 = l1;
                                    let len3 = l2;
                                    plugin::wit_bindgen::rt::dealloc(
                                        base3,
                                        (len3 as usize) * 32,
                                        8,
                                    );
                                }
                                _ => (),
                            }
                        }
                    };
                };
                const _: () = {
                    #[doc(hidden)]
                    #[export_name = "litehouse:plugin/plugin#[method]runner.update"]
                    #[allow(non_snake_case)]
                    unsafe extern "C" fn __export_method_runner_update(
                        arg0: i32,
                        arg1: i32,
                        arg2: i32,
                    ) -> i32 {
                        #[allow(unused_imports)]
                        use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                        #[cfg(target_arch = "wasm32")]
                        plugin::wit_bindgen::rt::run_ctors_once();
                        let base7 = arg1;
                        let len7 = arg2;
                        let mut result7 = Vec::with_capacity(len7 as usize);
                        for i in 0..len7 {
                            let base = base7 + i * 32;
                            let e7 = {
                                let l0 = *((base + 0) as *const i64);
                                let l1 = *((base + 8) as *const i64);
                                let l2 = i32::from(*((base + 16) as *const u8));
                                let v6 = match l2 {
                                    0 => {
                                        let e6 = {
                                            let l3 = *((base + 24) as *const i64);
                                            l3 as u64
                                        };
                                        Update::Time(e6)
                                    }
                                    1 => {
                                        let e6 = {
                                            let l4 = *((base + 24) as *const f64);
                                            l4
                                        };
                                        Update::Temperature(e6)
                                    }
                                    n => {
                                        if true {
                                            match (&n, &2) {
                                                (left_val, right_val) => {
                                                    if !(*left_val == *right_val) {
                                                        let kind =
                                                            ::core::panicking::AssertKind::Eq;
                                                        ::core::panicking::assert_failed(
                                                            kind,
                                                            &*left_val,
                                                            &*right_val,
                                                            ::core::option::Option::Some(
                                                                format_args!(
                                                                    "invalid enum discriminant"
                                                                ),
                                                            ),
                                                        );
                                                    }
                                                }
                                            };
                                        };
                                        let e6 = {
                                            let l5 = *((base + 24) as *const f64);
                                            l5
                                        };
                                        Update::WindSpeed(e6)
                                    }
                                };
                                Event {
                                    id: l0 as u64,
                                    timestamp: l1 as u64,
                                    inner: v6,
                                }
                            };
                            result7.push(e7);
                        }
                        plugin::wit_bindgen::rt::dealloc(base7, (len7 as usize) * 32, 8);
                        let result8 = <_RunnerImpl as GuestRunner>::update(
                            plugin::wit_bindgen::rt::Resource::<Runner>::lift_borrow(
                                arg0 as u32 as usize,
                            ),
                            result7,
                        );
                        let ptr9 = _RET_AREA.0.as_mut_ptr() as i32;
                        match result8 {
                            Ok(e) => {
                                *((ptr9 + 0) as *mut u8) = (0i32) as u8;
                                *((ptr9 + 4) as *mut u8) = (match e {
                                    true => 1,
                                    false => 0,
                                }) as u8;
                            }
                            Err(e) => {
                                *((ptr9 + 0) as *mut u8) = (1i32) as u8;
                                *((ptr9 + 4) as *mut i32) = plugin::wit_bindgen::rt::as_i32(e);
                            }
                        };
                        ptr9
                    }
                };
                const _: () = {
                    #[doc(hidden)]
                    #[export_name = "litehouse:plugin/plugin#generate-config-schema"]
                    #[allow(non_snake_case)]
                    unsafe extern "C" fn __export_generate_config_schema() -> i32 {
                        #[allow(unused_imports)]
                        use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                        #[cfg(target_arch = "wasm32")]
                        plugin::wit_bindgen::rt::run_ctors_once();
                        let result0 = <_GuestImpl as Guest>::generate_config_schema();
                        let ptr1 = _RET_AREA.0.as_mut_ptr() as i32;
                        match result0 {
                            Some(e) => {
                                *((ptr1 + 0) as *mut u8) = (1i32) as u8;
                                let vec2 = (e.into_bytes()).into_boxed_slice();
                                let ptr2 = vec2.as_ptr() as i32;
                                let len2 = vec2.len() as i32;
                                ::core::mem::forget(vec2);
                                *((ptr1 + 8) as *mut i32) = len2;
                                *((ptr1 + 4) as *mut i32) = ptr2;
                            }
                            None => {
                                *((ptr1 + 0) as *mut u8) = (0i32) as u8;
                            }
                        };
                        ptr1
                    }
                    const _: () = {
                        #[doc(hidden)]
                        #[export_name = "cabi_post_litehouse:plugin/plugin#generate-config-schema"]
                        #[allow(non_snake_case)]
                        unsafe extern "C" fn __post_return_generate_config_schema(arg0: i32) {
                            let l0 = i32::from(*((arg0 + 0) as *const u8));
                            match l0 {
                                0 => (),
                                _ => {
                                    let l1 = *((arg0 + 4) as *const i32);
                                    let l2 = *((arg0 + 8) as *const i32);
                                    plugin::wit_bindgen::rt::dealloc(l1, (l2) as usize, 1);
                                }
                            }
                        }
                    };
                };
                use super::super::super::super::TasmotaConfig as _GuestImpl;
                pub trait Guest {
                    fn generate_config_schema() -> Option<plugin::wit_bindgen::rt::string::String>;
                }
                use super::super::super::super::TasmotaPlugin as _RunnerImpl;
                pub trait GuestRunner {
                    fn new() -> Self;
                    fn subscribe(
                        &self,
                    ) -> Result<plugin::wit_bindgen::rt::vec::Vec<Subscription>, u32>;
                    fn update(
                        &self,
                        events: plugin::wit_bindgen::rt::vec::Vec<Event>,
                    ) -> Result<bool, u32>;
                }
                #[allow(unused_imports)]
                use plugin::wit_bindgen::rt::{alloc, vec::Vec, string::String};
                #[repr(align(4))]
                struct _RetArea([u8; 12]);
                static mut _RET_AREA: _RetArea = _RetArea([0; 12]);
            }
        }
    }
}
#[cfg(target_arch = "wasm32")]
#[link_section = "component-type:plugin-host"]
#[doc(hidden)]
pub static __WIT_BINDGEN_COMPONENT_TYPE: [u8; 8039] = [
    3, 0, 11, 112, 108, 117, 103, 105, 110, 45, 104, 111, 115, 116, 0, 97, 115, 109, 13, 0, 1, 0,
    7, 219, 3, 1, 65, 2, 1, 66, 28, 1, 113, 3, 4, 116, 105, 109, 101, 1, 119, 0, 11, 116, 101, 109,
    112, 101, 114, 97, 116, 117, 114, 101, 1, 117, 0, 10, 119, 105, 110, 100, 45, 115, 112, 101,
    101, 100, 1, 117, 0, 4, 0, 6, 117, 112, 100, 97, 116, 101, 3, 0, 0, 1, 114, 3, 2, 105, 100,
    119, 9, 116, 105, 109, 101, 115, 116, 97, 109, 112, 119, 5, 105, 110, 110, 101, 114, 1, 4, 0,
    5, 101, 118, 101, 110, 116, 3, 0, 2, 1, 113, 7, 6, 115, 101, 99, 111, 110, 100, 0, 0, 6, 109,
    105, 110, 117, 116, 101, 0, 0, 4, 104, 111, 117, 114, 0, 0, 3, 100, 97, 121, 0, 0, 4, 119, 101,
    101, 107, 0, 0, 5, 109, 111, 110, 116, 104, 0, 0, 4, 121, 101, 97, 114, 0, 0, 4, 0, 9, 116,
    105, 109, 101, 45, 117, 110, 105, 116, 3, 0, 4, 1, 114, 2, 6, 97, 109, 111, 117, 110, 116, 119,
    4, 117, 110, 105, 116, 5, 4, 0, 5, 101, 118, 101, 114, 121, 3, 0, 6, 1, 113, 2, 5, 101, 118,
    101, 114, 121, 1, 7, 0, 2, 97, 116, 1, 119, 0, 4, 0, 17, 116, 105, 109, 101, 45, 115, 117, 98,
    115, 99, 114, 105, 112, 116, 105, 111, 110, 3, 0, 8, 1, 113, 1, 4, 116, 105, 109, 101, 1, 9, 0,
    4, 0, 12, 115, 117, 98, 115, 99, 114, 105, 112, 116, 105, 111, 110, 3, 0, 10, 4, 0, 6, 114,
    117, 110, 110, 101, 114, 3, 1, 1, 105, 12, 1, 64, 0, 0, 13, 4, 0, 19, 91, 99, 111, 110, 115,
    116, 114, 117, 99, 116, 111, 114, 93, 114, 117, 110, 110, 101, 114, 1, 14, 1, 104, 12, 1, 112,
    11, 1, 106, 1, 16, 1, 121, 1, 64, 1, 4, 115, 101, 108, 102, 15, 0, 17, 4, 0, 24, 91, 109, 101,
    116, 104, 111, 100, 93, 114, 117, 110, 110, 101, 114, 46, 115, 117, 98, 115, 99, 114, 105, 98,
    101, 1, 18, 1, 112, 3, 1, 106, 1, 127, 1, 121, 1, 64, 2, 4, 115, 101, 108, 102, 15, 6, 101,
    118, 101, 110, 116, 115, 19, 0, 20, 4, 0, 21, 91, 109, 101, 116, 104, 111, 100, 93, 114, 117,
    110, 110, 101, 114, 46, 117, 112, 100, 97, 116, 101, 1, 21, 1, 107, 115, 1, 64, 0, 0, 22, 4, 0,
    22, 103, 101, 110, 101, 114, 97, 116, 101, 45, 99, 111, 110, 102, 105, 103, 45, 115, 99, 104,
    101, 109, 97, 1, 23, 4, 1, 23, 108, 105, 116, 101, 104, 111, 117, 115, 101, 58, 112, 108, 117,
    103, 105, 110, 47, 112, 108, 117, 103, 105, 110, 5, 0, 11, 12, 1, 0, 6, 112, 108, 117, 103,
    105, 110, 3, 0, 0, 7, 245, 57, 1, 65, 2, 1, 65, 31, 1, 66, 10, 4, 0, 8, 112, 111, 108, 108, 97,
    98, 108, 101, 3, 1, 1, 104, 0, 1, 64, 1, 4, 115, 101, 108, 102, 1, 0, 127, 4, 0, 22, 91, 109,
    101, 116, 104, 111, 100, 93, 112, 111, 108, 108, 97, 98, 108, 101, 46, 114, 101, 97, 100, 121,
    1, 2, 1, 64, 1, 4, 115, 101, 108, 102, 1, 1, 0, 4, 0, 22, 91, 109, 101, 116, 104, 111, 100, 93,
    112, 111, 108, 108, 97, 98, 108, 101, 46, 98, 108, 111, 99, 107, 1, 3, 1, 112, 1, 1, 112, 121,
    1, 64, 1, 2, 105, 110, 4, 0, 5, 4, 0, 4, 112, 111, 108, 108, 1, 6, 3, 1, 32, 119, 97, 115, 105,
    58, 105, 111, 47, 112, 111, 108, 108, 64, 48, 46, 50, 46, 48, 45, 114, 99, 45, 50, 48, 50, 51,
    45, 49, 49, 45, 49, 48, 5, 0, 2, 3, 0, 0, 8, 112, 111, 108, 108, 97, 98, 108, 101, 1, 66, 15,
    2, 3, 2, 1, 1, 4, 0, 8, 112, 111, 108, 108, 97, 98, 108, 101, 3, 0, 0, 1, 119, 4, 0, 7, 105,
    110, 115, 116, 97, 110, 116, 3, 0, 2, 1, 119, 4, 0, 8, 100, 117, 114, 97, 116, 105, 111, 110,
    3, 0, 4, 1, 64, 0, 0, 3, 4, 0, 3, 110, 111, 119, 1, 6, 1, 64, 0, 0, 5, 4, 0, 10, 114, 101, 115,
    111, 108, 117, 116, 105, 111, 110, 1, 7, 1, 105, 1, 1, 64, 1, 4, 119, 104, 101, 110, 3, 0, 8,
    4, 0, 17, 115, 117, 98, 115, 99, 114, 105, 98, 101, 45, 105, 110, 115, 116, 97, 110, 116, 1, 9,
    1, 64, 1, 4, 119, 104, 101, 110, 5, 0, 8, 4, 0, 18, 115, 117, 98, 115, 99, 114, 105, 98, 101,
    45, 100, 117, 114, 97, 116, 105, 111, 110, 1, 10, 3, 1, 47, 119, 97, 115, 105, 58, 99, 108,
    111, 99, 107, 115, 47, 109, 111, 110, 111, 116, 111, 110, 105, 99, 45, 99, 108, 111, 99, 107,
    64, 48, 46, 50, 46, 48, 45, 114, 99, 45, 50, 48, 50, 51, 45, 49, 49, 45, 49, 48, 5, 2, 1, 66,
    4, 4, 0, 5, 101, 114, 114, 111, 114, 3, 1, 1, 104, 0, 1, 64, 1, 4, 115, 101, 108, 102, 1, 0,
    115, 4, 0, 29, 91, 109, 101, 116, 104, 111, 100, 93, 101, 114, 114, 111, 114, 46, 116, 111, 45,
    100, 101, 98, 117, 103, 45, 115, 116, 114, 105, 110, 103, 1, 2, 3, 1, 33, 119, 97, 115, 105,
    58, 105, 111, 47, 101, 114, 114, 111, 114, 64, 48, 46, 50, 46, 48, 45, 114, 99, 45, 50, 48, 50,
    51, 45, 49, 49, 45, 49, 48, 5, 3, 2, 3, 0, 2, 5, 101, 114, 114, 111, 114, 1, 66, 40, 2, 3, 2,
    1, 4, 4, 0, 5, 101, 114, 114, 111, 114, 3, 0, 0, 2, 3, 2, 1, 1, 4, 0, 8, 112, 111, 108, 108,
    97, 98, 108, 101, 3, 0, 2, 1, 105, 1, 1, 113, 2, 21, 108, 97, 115, 116, 45, 111, 112, 101, 114,
    97, 116, 105, 111, 110, 45, 102, 97, 105, 108, 101, 100, 1, 4, 0, 6, 99, 108, 111, 115, 101,
    100, 0, 0, 4, 0, 12, 115, 116, 114, 101, 97, 109, 45, 101, 114, 114, 111, 114, 3, 0, 5, 4, 0,
    12, 105, 110, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 3, 1, 4, 0, 13, 111, 117, 116,
    112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 3, 1, 1, 104, 7, 1, 112, 125, 1, 106, 1, 10, 1,
    6, 1, 64, 2, 4, 115, 101, 108, 102, 9, 3, 108, 101, 110, 119, 0, 11, 4, 0, 25, 91, 109, 101,
    116, 104, 111, 100, 93, 105, 110, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 46, 114, 101,
    97, 100, 1, 12, 4, 0, 34, 91, 109, 101, 116, 104, 111, 100, 93, 105, 110, 112, 117, 116, 45,
    115, 116, 114, 101, 97, 109, 46, 98, 108, 111, 99, 107, 105, 110, 103, 45, 114, 101, 97, 100,
    1, 12, 1, 106, 1, 119, 1, 6, 1, 64, 2, 4, 115, 101, 108, 102, 9, 3, 108, 101, 110, 119, 0, 13,
    4, 0, 25, 91, 109, 101, 116, 104, 111, 100, 93, 105, 110, 112, 117, 116, 45, 115, 116, 114,
    101, 97, 109, 46, 115, 107, 105, 112, 1, 14, 4, 0, 34, 91, 109, 101, 116, 104, 111, 100, 93,
    105, 110, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 46, 98, 108, 111, 99, 107, 105, 110,
    103, 45, 115, 107, 105, 112, 1, 14, 1, 105, 3, 1, 64, 1, 4, 115, 101, 108, 102, 9, 0, 15, 4, 0,
    30, 91, 109, 101, 116, 104, 111, 100, 93, 105, 110, 112, 117, 116, 45, 115, 116, 114, 101, 97,
    109, 46, 115, 117, 98, 115, 99, 114, 105, 98, 101, 1, 16, 1, 104, 8, 1, 64, 1, 4, 115, 101,
    108, 102, 17, 0, 13, 4, 0, 33, 91, 109, 101, 116, 104, 111, 100, 93, 111, 117, 116, 112, 117,
    116, 45, 115, 116, 114, 101, 97, 109, 46, 99, 104, 101, 99, 107, 45, 119, 114, 105, 116, 101,
    1, 18, 1, 106, 0, 1, 6, 1, 64, 2, 4, 115, 101, 108, 102, 17, 8, 99, 111, 110, 116, 101, 110,
    116, 115, 10, 0, 19, 4, 0, 27, 91, 109, 101, 116, 104, 111, 100, 93, 111, 117, 116, 112, 117,
    116, 45, 115, 116, 114, 101, 97, 109, 46, 119, 114, 105, 116, 101, 1, 20, 4, 0, 46, 91, 109,
    101, 116, 104, 111, 100, 93, 111, 117, 116, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 46,
    98, 108, 111, 99, 107, 105, 110, 103, 45, 119, 114, 105, 116, 101, 45, 97, 110, 100, 45, 102,
    108, 117, 115, 104, 1, 20, 1, 64, 1, 4, 115, 101, 108, 102, 17, 0, 19, 4, 0, 27, 91, 109, 101,
    116, 104, 111, 100, 93, 111, 117, 116, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 46, 102,
    108, 117, 115, 104, 1, 21, 4, 0, 36, 91, 109, 101, 116, 104, 111, 100, 93, 111, 117, 116, 112,
    117, 116, 45, 115, 116, 114, 101, 97, 109, 46, 98, 108, 111, 99, 107, 105, 110, 103, 45, 102,
    108, 117, 115, 104, 1, 21, 1, 64, 1, 4, 115, 101, 108, 102, 17, 0, 15, 4, 0, 31, 91, 109, 101,
    116, 104, 111, 100, 93, 111, 117, 116, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 46, 115,
    117, 98, 115, 99, 114, 105, 98, 101, 1, 22, 1, 64, 2, 4, 115, 101, 108, 102, 17, 3, 108, 101,
    110, 119, 0, 19, 4, 0, 34, 91, 109, 101, 116, 104, 111, 100, 93, 111, 117, 116, 112, 117, 116,
    45, 115, 116, 114, 101, 97, 109, 46, 119, 114, 105, 116, 101, 45, 122, 101, 114, 111, 101, 115,
    1, 23, 4, 0, 53, 91, 109, 101, 116, 104, 111, 100, 93, 111, 117, 116, 112, 117, 116, 45, 115,
    116, 114, 101, 97, 109, 46, 98, 108, 111, 99, 107, 105, 110, 103, 45, 119, 114, 105, 116, 101,
    45, 122, 101, 114, 111, 101, 115, 45, 97, 110, 100, 45, 102, 108, 117, 115, 104, 1, 23, 1, 64,
    3, 4, 115, 101, 108, 102, 17, 3, 115, 114, 99, 9, 3, 108, 101, 110, 119, 0, 13, 4, 0, 28, 91,
    109, 101, 116, 104, 111, 100, 93, 111, 117, 116, 112, 117, 116, 45, 115, 116, 114, 101, 97,
    109, 46, 115, 112, 108, 105, 99, 101, 1, 24, 4, 0, 37, 91, 109, 101, 116, 104, 111, 100, 93,
    111, 117, 116, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 46, 98, 108, 111, 99, 107, 105,
    110, 103, 45, 115, 112, 108, 105, 99, 101, 1, 24, 3, 1, 35, 119, 97, 115, 105, 58, 105, 111,
    47, 115, 116, 114, 101, 97, 109, 115, 64, 48, 46, 50, 46, 48, 45, 114, 99, 45, 50, 48, 50, 51,
    45, 49, 49, 45, 49, 48, 5, 5, 2, 3, 0, 1, 8, 100, 117, 114, 97, 116, 105, 111, 110, 2, 3, 0, 3,
    12, 105, 110, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 2, 3, 0, 3, 13, 111, 117, 116,
    112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 1, 66, 192, 1, 2, 3, 2, 1, 6, 4, 0, 8, 100,
    117, 114, 97, 116, 105, 111, 110, 3, 0, 0, 2, 3, 2, 1, 7, 4, 0, 12, 105, 110, 112, 117, 116,
    45, 115, 116, 114, 101, 97, 109, 3, 0, 2, 2, 3, 2, 1, 8, 4, 0, 13, 111, 117, 116, 112, 117,
    116, 45, 115, 116, 114, 101, 97, 109, 3, 0, 4, 2, 3, 2, 1, 4, 4, 0, 8, 105, 111, 45, 101, 114,
    114, 111, 114, 3, 0, 6, 2, 3, 2, 1, 1, 4, 0, 8, 112, 111, 108, 108, 97, 98, 108, 101, 3, 0, 8,
    1, 113, 10, 3, 103, 101, 116, 0, 0, 4, 104, 101, 97, 100, 0, 0, 4, 112, 111, 115, 116, 0, 0, 3,
    112, 117, 116, 0, 0, 6, 100, 101, 108, 101, 116, 101, 0, 0, 7, 99, 111, 110, 110, 101, 99, 116,
    0, 0, 7, 111, 112, 116, 105, 111, 110, 115, 0, 0, 5, 116, 114, 97, 99, 101, 0, 0, 5, 112, 97,
    116, 99, 104, 0, 0, 5, 111, 116, 104, 101, 114, 1, 115, 0, 4, 0, 6, 109, 101, 116, 104, 111,
    100, 3, 0, 10, 1, 113, 3, 4, 72, 84, 84, 80, 0, 0, 5, 72, 84, 84, 80, 83, 0, 0, 5, 111, 116,
    104, 101, 114, 1, 115, 0, 4, 0, 6, 115, 99, 104, 101, 109, 101, 3, 0, 12, 1, 107, 115, 1, 107,
    123, 1, 114, 2, 5, 114, 99, 111, 100, 101, 14, 9, 105, 110, 102, 111, 45, 99, 111, 100, 101,
    15, 4, 0, 17, 68, 78, 83, 45, 101, 114, 114, 111, 114, 45, 112, 97, 121, 108, 111, 97, 100, 3,
    0, 16, 1, 107, 125, 1, 114, 2, 8, 97, 108, 101, 114, 116, 45, 105, 100, 18, 13, 97, 108, 101,
    114, 116, 45, 109, 101, 115, 115, 97, 103, 101, 14, 4, 0, 26, 84, 76, 83, 45, 97, 108, 101,
    114, 116, 45, 114, 101, 99, 101, 105, 118, 101, 100, 45, 112, 97, 121, 108, 111, 97, 100, 3, 0,
    19, 1, 107, 121, 1, 114, 2, 10, 102, 105, 101, 108, 100, 45, 110, 97, 109, 101, 14, 10, 102,
    105, 101, 108, 100, 45, 115, 105, 122, 101, 21, 4, 0, 18, 102, 105, 101, 108, 100, 45, 115,
    105, 122, 101, 45, 112, 97, 121, 108, 111, 97, 100, 3, 0, 22, 1, 107, 119, 1, 107, 23, 1, 113,
    39, 11, 68, 78, 83, 45, 116, 105, 109, 101, 111, 117, 116, 0, 0, 9, 68, 78, 83, 45, 101, 114,
    114, 111, 114, 1, 17, 0, 21, 100, 101, 115, 116, 105, 110, 97, 116, 105, 111, 110, 45, 110,
    111, 116, 45, 102, 111, 117, 110, 100, 0, 0, 23, 100, 101, 115, 116, 105, 110, 97, 116, 105,
    111, 110, 45, 117, 110, 97, 118, 97, 105, 108, 97, 98, 108, 101, 0, 0, 25, 100, 101, 115, 116,
    105, 110, 97, 116, 105, 111, 110, 45, 73, 80, 45, 112, 114, 111, 104, 105, 98, 105, 116, 101,
    100, 0, 0, 25, 100, 101, 115, 116, 105, 110, 97, 116, 105, 111, 110, 45, 73, 80, 45, 117, 110,
    114, 111, 117, 116, 97, 98, 108, 101, 0, 0, 18, 99, 111, 110, 110, 101, 99, 116, 105, 111, 110,
    45, 114, 101, 102, 117, 115, 101, 100, 0, 0, 21, 99, 111, 110, 110, 101, 99, 116, 105, 111,
    110, 45, 116, 101, 114, 109, 105, 110, 97, 116, 101, 100, 0, 0, 18, 99, 111, 110, 110, 101, 99,
    116, 105, 111, 110, 45, 116, 105, 109, 101, 111, 117, 116, 0, 0, 23, 99, 111, 110, 110, 101,
    99, 116, 105, 111, 110, 45, 114, 101, 97, 100, 45, 116, 105, 109, 101, 111, 117, 116, 0, 0, 24,
    99, 111, 110, 110, 101, 99, 116, 105, 111, 110, 45, 119, 114, 105, 116, 101, 45, 116, 105, 109,
    101, 111, 117, 116, 0, 0, 24, 99, 111, 110, 110, 101, 99, 116, 105, 111, 110, 45, 108, 105,
    109, 105, 116, 45, 114, 101, 97, 99, 104, 101, 100, 0, 0, 18, 84, 76, 83, 45, 112, 114, 111,
    116, 111, 99, 111, 108, 45, 101, 114, 114, 111, 114, 0, 0, 21, 84, 76, 83, 45, 99, 101, 114,
    116, 105, 102, 105, 99, 97, 116, 101, 45, 101, 114, 114, 111, 114, 0, 0, 18, 84, 76, 83, 45,
    97, 108, 101, 114, 116, 45, 114, 101, 99, 101, 105, 118, 101, 100, 1, 20, 0, 19, 72, 84, 84,
    80, 45, 114, 101, 113, 117, 101, 115, 116, 45, 100, 101, 110, 105, 101, 100, 0, 0, 28, 72, 84,
    84, 80, 45, 114, 101, 113, 117, 101, 115, 116, 45, 108, 101, 110, 103, 116, 104, 45, 114, 101,
    113, 117, 105, 114, 101, 100, 0, 0, 22, 72, 84, 84, 80, 45, 114, 101, 113, 117, 101, 115, 116,
    45, 98, 111, 100, 121, 45, 115, 105, 122, 101, 1, 24, 0, 27, 72, 84, 84, 80, 45, 114, 101, 113,
    117, 101, 115, 116, 45, 109, 101, 116, 104, 111, 100, 45, 105, 110, 118, 97, 108, 105, 100, 0,
    0, 24, 72, 84, 84, 80, 45, 114, 101, 113, 117, 101, 115, 116, 45, 85, 82, 73, 45, 105, 110,
    118, 97, 108, 105, 100, 0, 0, 25, 72, 84, 84, 80, 45, 114, 101, 113, 117, 101, 115, 116, 45,
    85, 82, 73, 45, 116, 111, 111, 45, 108, 111, 110, 103, 0, 0, 32, 72, 84, 84, 80, 45, 114, 101,
    113, 117, 101, 115, 116, 45, 104, 101, 97, 100, 101, 114, 45, 115, 101, 99, 116, 105, 111, 110,
    45, 115, 105, 122, 101, 1, 21, 0, 24, 72, 84, 84, 80, 45, 114, 101, 113, 117, 101, 115, 116,
    45, 104, 101, 97, 100, 101, 114, 45, 115, 105, 122, 101, 1, 25, 0, 33, 72, 84, 84, 80, 45, 114,
    101, 113, 117, 101, 115, 116, 45, 116, 114, 97, 105, 108, 101, 114, 45, 115, 101, 99, 116, 105,
    111, 110, 45, 115, 105, 122, 101, 1, 21, 0, 25, 72, 84, 84, 80, 45, 114, 101, 113, 117, 101,
    115, 116, 45, 116, 114, 97, 105, 108, 101, 114, 45, 115, 105, 122, 101, 1, 23, 0, 24, 72, 84,
    84, 80, 45, 114, 101, 115, 112, 111, 110, 115, 101, 45, 105, 110, 99, 111, 109, 112, 108, 101,
    116, 101, 0, 0, 33, 72, 84, 84, 80, 45, 114, 101, 115, 112, 111, 110, 115, 101, 45, 104, 101,
    97, 100, 101, 114, 45, 115, 101, 99, 116, 105, 111, 110, 45, 115, 105, 122, 101, 1, 21, 0, 25,
    72, 84, 84, 80, 45, 114, 101, 115, 112, 111, 110, 115, 101, 45, 104, 101, 97, 100, 101, 114,
    45, 115, 105, 122, 101, 1, 23, 0, 23, 72, 84, 84, 80, 45, 114, 101, 115, 112, 111, 110, 115,
    101, 45, 98, 111, 100, 121, 45, 115, 105, 122, 101, 1, 24, 0, 34, 72, 84, 84, 80, 45, 114, 101,
    115, 112, 111, 110, 115, 101, 45, 116, 114, 97, 105, 108, 101, 114, 45, 115, 101, 99, 116, 105,
    111, 110, 45, 115, 105, 122, 101, 1, 21, 0, 26, 72, 84, 84, 80, 45, 114, 101, 115, 112, 111,
    110, 115, 101, 45, 116, 114, 97, 105, 108, 101, 114, 45, 115, 105, 122, 101, 1, 23, 0, 29, 72,
    84, 84, 80, 45, 114, 101, 115, 112, 111, 110, 115, 101, 45, 116, 114, 97, 110, 115, 102, 101,
    114, 45, 99, 111, 100, 105, 110, 103, 1, 14, 0, 28, 72, 84, 84, 80, 45, 114, 101, 115, 112,
    111, 110, 115, 101, 45, 99, 111, 110, 116, 101, 110, 116, 45, 99, 111, 100, 105, 110, 103, 1,
    14, 0, 21, 72, 84, 84, 80, 45, 114, 101, 115, 112, 111, 110, 115, 101, 45, 116, 105, 109, 101,
    111, 117, 116, 0, 0, 19, 72, 84, 84, 80, 45, 117, 112, 103, 114, 97, 100, 101, 45, 102, 97,
    105, 108, 101, 100, 0, 0, 19, 72, 84, 84, 80, 45, 112, 114, 111, 116, 111, 99, 111, 108, 45,
    101, 114, 114, 111, 114, 0, 0, 13, 108, 111, 111, 112, 45, 100, 101, 116, 101, 99, 116, 101,
    100, 0, 0, 19, 99, 111, 110, 102, 105, 103, 117, 114, 97, 116, 105, 111, 110, 45, 101, 114,
    114, 111, 114, 0, 0, 14, 105, 110, 116, 101, 114, 110, 97, 108, 45, 101, 114, 114, 111, 114, 1,
    14, 0, 4, 0, 10, 101, 114, 114, 111, 114, 45, 99, 111, 100, 101, 3, 0, 26, 1, 113, 3, 14, 105,
    110, 118, 97, 108, 105, 100, 45, 115, 121, 110, 116, 97, 120, 0, 0, 9, 102, 111, 114, 98, 105,
    100, 100, 101, 110, 0, 0, 9, 105, 109, 109, 117, 116, 97, 98, 108, 101, 0, 0, 4, 0, 12, 104,
    101, 97, 100, 101, 114, 45, 101, 114, 114, 111, 114, 3, 0, 28, 1, 115, 4, 0, 9, 102, 105, 101,
    108, 100, 45, 107, 101, 121, 3, 0, 30, 1, 112, 125, 4, 0, 11, 102, 105, 101, 108, 100, 45, 118,
    97, 108, 117, 101, 3, 0, 32, 4, 0, 6, 102, 105, 101, 108, 100, 115, 3, 1, 4, 0, 7, 104, 101,
    97, 100, 101, 114, 115, 3, 0, 34, 4, 0, 8, 116, 114, 97, 105, 108, 101, 114, 115, 3, 0, 34, 4,
    0, 16, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 3, 1, 4,
    0, 16, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 3, 1, 4,
    0, 15, 114, 101, 113, 117, 101, 115, 116, 45, 111, 112, 116, 105, 111, 110, 115, 3, 1, 4, 0,
    17, 114, 101, 115, 112, 111, 110, 115, 101, 45, 111, 117, 116, 112, 97, 114, 97, 109, 3, 1, 1,
    123, 4, 0, 11, 115, 116, 97, 116, 117, 115, 45, 99, 111, 100, 101, 3, 0, 41, 4, 0, 17, 105,
    110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 3, 1, 4, 0, 13,
    105, 110, 99, 111, 109, 105, 110, 103, 45, 98, 111, 100, 121, 3, 1, 4, 0, 15, 102, 117, 116,
    117, 114, 101, 45, 116, 114, 97, 105, 108, 101, 114, 115, 3, 1, 4, 0, 17, 111, 117, 116, 103,
    111, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 3, 1, 4, 0, 13, 111, 117, 116,
    103, 111, 105, 110, 103, 45, 98, 111, 100, 121, 3, 1, 4, 0, 24, 102, 117, 116, 117, 114, 101,
    45, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 3, 1, 1,
    105, 34, 1, 64, 0, 0, 49, 4, 0, 19, 91, 99, 111, 110, 115, 116, 114, 117, 99, 116, 111, 114,
    93, 102, 105, 101, 108, 100, 115, 1, 50, 1, 111, 2, 31, 33, 1, 112, 51, 1, 106, 1, 49, 1, 29,
    1, 64, 1, 7, 101, 110, 116, 114, 105, 101, 115, 52, 0, 53, 4, 0, 24, 91, 115, 116, 97, 116,
    105, 99, 93, 102, 105, 101, 108, 100, 115, 46, 102, 114, 111, 109, 45, 108, 105, 115, 116, 1,
    54, 1, 104, 34, 1, 112, 33, 1, 64, 2, 4, 115, 101, 108, 102, 55, 4, 110, 97, 109, 101, 31, 0,
    56, 4, 0, 18, 91, 109, 101, 116, 104, 111, 100, 93, 102, 105, 101, 108, 100, 115, 46, 103, 101,
    116, 1, 57, 1, 64, 2, 4, 115, 101, 108, 102, 55, 4, 110, 97, 109, 101, 31, 0, 127, 4, 0, 18,
    91, 109, 101, 116, 104, 111, 100, 93, 102, 105, 101, 108, 100, 115, 46, 104, 97, 115, 1, 58, 1,
    106, 0, 1, 29, 1, 64, 3, 4, 115, 101, 108, 102, 55, 4, 110, 97, 109, 101, 31, 5, 118, 97, 108,
    117, 101, 56, 0, 59, 4, 0, 18, 91, 109, 101, 116, 104, 111, 100, 93, 102, 105, 101, 108, 100,
    115, 46, 115, 101, 116, 1, 60, 1, 64, 2, 4, 115, 101, 108, 102, 55, 4, 110, 97, 109, 101, 31,
    0, 59, 4, 0, 21, 91, 109, 101, 116, 104, 111, 100, 93, 102, 105, 101, 108, 100, 115, 46, 100,
    101, 108, 101, 116, 101, 1, 61, 1, 64, 3, 4, 115, 101, 108, 102, 55, 4, 110, 97, 109, 101, 31,
    5, 118, 97, 108, 117, 101, 33, 0, 59, 4, 0, 21, 91, 109, 101, 116, 104, 111, 100, 93, 102, 105,
    101, 108, 100, 115, 46, 97, 112, 112, 101, 110, 100, 1, 62, 1, 64, 1, 4, 115, 101, 108, 102,
    55, 0, 52, 4, 0, 22, 91, 109, 101, 116, 104, 111, 100, 93, 102, 105, 101, 108, 100, 115, 46,
    101, 110, 116, 114, 105, 101, 115, 1, 63, 1, 64, 1, 4, 115, 101, 108, 102, 55, 0, 49, 4, 0, 20,
    91, 109, 101, 116, 104, 111, 100, 93, 102, 105, 101, 108, 100, 115, 46, 99, 108, 111, 110, 101,
    1, 64, 1, 104, 37, 1, 64, 1, 4, 115, 101, 108, 102, 193, 0, 0, 11, 4, 0, 31, 91, 109, 101, 116,
    104, 111, 100, 93, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115,
    116, 46, 109, 101, 116, 104, 111, 100, 1, 66, 1, 64, 1, 4, 115, 101, 108, 102, 193, 0, 0, 14,
    4, 0, 40, 91, 109, 101, 116, 104, 111, 100, 93, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114,
    101, 113, 117, 101, 115, 116, 46, 112, 97, 116, 104, 45, 119, 105, 116, 104, 45, 113, 117, 101,
    114, 121, 1, 67, 1, 107, 13, 1, 64, 1, 4, 115, 101, 108, 102, 193, 0, 0, 196, 0, 4, 0, 31, 91,
    109, 101, 116, 104, 111, 100, 93, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113,
    117, 101, 115, 116, 46, 115, 99, 104, 101, 109, 101, 1, 69, 4, 0, 34, 91, 109, 101, 116, 104,
    111, 100, 93, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 46,
    97, 117, 116, 104, 111, 114, 105, 116, 121, 1, 67, 1, 105, 35, 1, 64, 1, 4, 115, 101, 108, 102,
    193, 0, 0, 198, 0, 4, 0, 32, 91, 109, 101, 116, 104, 111, 100, 93, 105, 110, 99, 111, 109, 105,
    110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 46, 104, 101, 97, 100, 101, 114, 115, 1, 71,
    1, 105, 44, 1, 106, 1, 200, 0, 0, 1, 64, 1, 4, 115, 101, 108, 102, 193, 0, 0, 201, 0, 4, 0, 32,
    91, 109, 101, 116, 104, 111, 100, 93, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113,
    117, 101, 115, 116, 46, 99, 111, 110, 115, 117, 109, 101, 1, 74, 1, 105, 38, 1, 64, 1, 7, 104,
    101, 97, 100, 101, 114, 115, 198, 0, 0, 203, 0, 4, 0, 29, 91, 99, 111, 110, 115, 116, 114, 117,
    99, 116, 111, 114, 93, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 113, 117, 101,
    115, 116, 1, 76, 1, 104, 38, 1, 105, 47, 1, 106, 1, 206, 0, 0, 1, 64, 1, 4, 115, 101, 108, 102,
    205, 0, 0, 207, 0, 4, 0, 29, 91, 109, 101, 116, 104, 111, 100, 93, 111, 117, 116, 103, 111,
    105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 46, 98, 111, 100, 121, 1, 80, 1, 64, 1,
    4, 115, 101, 108, 102, 205, 0, 0, 11, 4, 0, 31, 91, 109, 101, 116, 104, 111, 100, 93, 111, 117,
    116, 103, 111, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 46, 109, 101, 116, 104,
    111, 100, 1, 81, 1, 106, 0, 0, 1, 64, 2, 4, 115, 101, 108, 102, 205, 0, 6, 109, 101, 116, 104,
    111, 100, 11, 0, 210, 0, 4, 0, 35, 91, 109, 101, 116, 104, 111, 100, 93, 111, 117, 116, 103,
    111, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 46, 115, 101, 116, 45, 109, 101,
    116, 104, 111, 100, 1, 83, 1, 64, 1, 4, 115, 101, 108, 102, 205, 0, 0, 14, 4, 0, 40, 91, 109,
    101, 116, 104, 111, 100, 93, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 113, 117,
    101, 115, 116, 46, 112, 97, 116, 104, 45, 119, 105, 116, 104, 45, 113, 117, 101, 114, 121, 1,
    84, 1, 64, 2, 4, 115, 101, 108, 102, 205, 0, 15, 112, 97, 116, 104, 45, 119, 105, 116, 104, 45,
    113, 117, 101, 114, 121, 14, 0, 210, 0, 4, 0, 44, 91, 109, 101, 116, 104, 111, 100, 93, 111,
    117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 46, 115, 101, 116,
    45, 112, 97, 116, 104, 45, 119, 105, 116, 104, 45, 113, 117, 101, 114, 121, 1, 85, 1, 64, 1, 4,
    115, 101, 108, 102, 205, 0, 0, 196, 0, 4, 0, 31, 91, 109, 101, 116, 104, 111, 100, 93, 111,
    117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 46, 115, 99, 104,
    101, 109, 101, 1, 86, 1, 64, 2, 4, 115, 101, 108, 102, 205, 0, 6, 115, 99, 104, 101, 109, 101,
    196, 0, 0, 210, 0, 4, 0, 35, 91, 109, 101, 116, 104, 111, 100, 93, 111, 117, 116, 103, 111,
    105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 46, 115, 101, 116, 45, 115, 99, 104, 101,
    109, 101, 1, 87, 4, 0, 34, 91, 109, 101, 116, 104, 111, 100, 93, 111, 117, 116, 103, 111, 105,
    110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 46, 97, 117, 116, 104, 111, 114, 105, 116,
    121, 1, 84, 1, 64, 2, 4, 115, 101, 108, 102, 205, 0, 9, 97, 117, 116, 104, 111, 114, 105, 116,
    121, 14, 0, 210, 0, 4, 0, 38, 91, 109, 101, 116, 104, 111, 100, 93, 111, 117, 116, 103, 111,
    105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 46, 115, 101, 116, 45, 97, 117, 116, 104,
    111, 114, 105, 116, 121, 1, 88, 1, 64, 1, 4, 115, 101, 108, 102, 205, 0, 0, 198, 0, 4, 0, 32,
    91, 109, 101, 116, 104, 111, 100, 93, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101,
    113, 117, 101, 115, 116, 46, 104, 101, 97, 100, 101, 114, 115, 1, 89, 1, 105, 39, 1, 64, 0, 0,
    218, 0, 4, 0, 28, 91, 99, 111, 110, 115, 116, 114, 117, 99, 116, 111, 114, 93, 114, 101, 113,
    117, 101, 115, 116, 45, 111, 112, 116, 105, 111, 110, 115, 1, 91, 1, 104, 39, 1, 107, 1, 1, 64,
    1, 4, 115, 101, 108, 102, 220, 0, 0, 221, 0, 4, 0, 39, 91, 109, 101, 116, 104, 111, 100, 93,
    114, 101, 113, 117, 101, 115, 116, 45, 111, 112, 116, 105, 111, 110, 115, 46, 99, 111, 110,
    110, 101, 99, 116, 45, 116, 105, 109, 101, 111, 117, 116, 1, 94, 1, 64, 2, 4, 115, 101, 108,
    102, 220, 0, 8, 100, 117, 114, 97, 116, 105, 111, 110, 221, 0, 0, 210, 0, 4, 0, 43, 91, 109,
    101, 116, 104, 111, 100, 93, 114, 101, 113, 117, 101, 115, 116, 45, 111, 112, 116, 105, 111,
    110, 115, 46, 115, 101, 116, 45, 99, 111, 110, 110, 101, 99, 116, 45, 116, 105, 109, 101, 111,
    117, 116, 1, 95, 4, 0, 42, 91, 109, 101, 116, 104, 111, 100, 93, 114, 101, 113, 117, 101, 115,
    116, 45, 111, 112, 116, 105, 111, 110, 115, 46, 102, 105, 114, 115, 116, 45, 98, 121, 116, 101,
    45, 116, 105, 109, 101, 111, 117, 116, 1, 94, 4, 0, 46, 91, 109, 101, 116, 104, 111, 100, 93,
    114, 101, 113, 117, 101, 115, 116, 45, 111, 112, 116, 105, 111, 110, 115, 46, 115, 101, 116,
    45, 102, 105, 114, 115, 116, 45, 98, 121, 116, 101, 45, 116, 105, 109, 101, 111, 117, 116, 1,
    95, 4, 0, 45, 91, 109, 101, 116, 104, 111, 100, 93, 114, 101, 113, 117, 101, 115, 116, 45, 111,
    112, 116, 105, 111, 110, 115, 46, 98, 101, 116, 119, 101, 101, 110, 45, 98, 121, 116, 101, 115,
    45, 116, 105, 109, 101, 111, 117, 116, 1, 94, 4, 0, 49, 91, 109, 101, 116, 104, 111, 100, 93,
    114, 101, 113, 117, 101, 115, 116, 45, 111, 112, 116, 105, 111, 110, 115, 46, 115, 101, 116,
    45, 98, 101, 116, 119, 101, 101, 110, 45, 98, 121, 116, 101, 115, 45, 116, 105, 109, 101, 111,
    117, 116, 1, 95, 1, 105, 40, 1, 105, 46, 1, 106, 1, 225, 0, 1, 27, 1, 64, 2, 5, 112, 97, 114,
    97, 109, 224, 0, 8, 114, 101, 115, 112, 111, 110, 115, 101, 226, 0, 1, 0, 4, 0, 29, 91, 115,
    116, 97, 116, 105, 99, 93, 114, 101, 115, 112, 111, 110, 115, 101, 45, 111, 117, 116, 112, 97,
    114, 97, 109, 46, 115, 101, 116, 1, 99, 1, 104, 43, 1, 64, 1, 4, 115, 101, 108, 102, 228, 0, 0,
    42, 4, 0, 32, 91, 109, 101, 116, 104, 111, 100, 93, 105, 110, 99, 111, 109, 105, 110, 103, 45,
    114, 101, 115, 112, 111, 110, 115, 101, 46, 115, 116, 97, 116, 117, 115, 1, 101, 1, 64, 1, 4,
    115, 101, 108, 102, 228, 0, 0, 198, 0, 4, 0, 33, 91, 109, 101, 116, 104, 111, 100, 93, 105,
    110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 46, 104, 101, 97,
    100, 101, 114, 115, 1, 102, 1, 64, 1, 4, 115, 101, 108, 102, 228, 0, 0, 201, 0, 4, 0, 33, 91,
    109, 101, 116, 104, 111, 100, 93, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 115,
    112, 111, 110, 115, 101, 46, 99, 111, 110, 115, 117, 109, 101, 1, 103, 1, 104, 44, 1, 105, 3,
    1, 106, 1, 233, 0, 0, 1, 64, 1, 4, 115, 101, 108, 102, 232, 0, 0, 234, 0, 4, 0, 28, 91, 109,
    101, 116, 104, 111, 100, 93, 105, 110, 99, 111, 109, 105, 110, 103, 45, 98, 111, 100, 121, 46,
    115, 116, 114, 101, 97, 109, 1, 107, 1, 105, 45, 1, 64, 1, 4, 116, 104, 105, 115, 200, 0, 0,
    236, 0, 4, 0, 28, 91, 115, 116, 97, 116, 105, 99, 93, 105, 110, 99, 111, 109, 105, 110, 103,
    45, 98, 111, 100, 121, 46, 102, 105, 110, 105, 115, 104, 1, 109, 1, 104, 45, 1, 105, 9, 1, 64,
    1, 4, 115, 101, 108, 102, 238, 0, 0, 239, 0, 4, 0, 33, 91, 109, 101, 116, 104, 111, 100, 93,
    102, 117, 116, 117, 114, 101, 45, 116, 114, 97, 105, 108, 101, 114, 115, 46, 115, 117, 98, 115,
    99, 114, 105, 98, 101, 1, 112, 1, 105, 36, 1, 107, 241, 0, 1, 106, 1, 242, 0, 1, 27, 1, 106, 1,
    243, 0, 0, 1, 107, 244, 0, 1, 64, 1, 4, 115, 101, 108, 102, 238, 0, 0, 245, 0, 4, 0, 27, 91,
    109, 101, 116, 104, 111, 100, 93, 102, 117, 116, 117, 114, 101, 45, 116, 114, 97, 105, 108,
    101, 114, 115, 46, 103, 101, 116, 1, 118, 1, 64, 1, 7, 104, 101, 97, 100, 101, 114, 115, 198,
    0, 0, 225, 0, 4, 0, 30, 91, 99, 111, 110, 115, 116, 114, 117, 99, 116, 111, 114, 93, 111, 117,
    116, 103, 111, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 1, 119, 1, 104, 46,
    1, 64, 1, 4, 115, 101, 108, 102, 248, 0, 0, 42, 4, 0, 37, 91, 109, 101, 116, 104, 111, 100, 93,
    111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 46, 115,
    116, 97, 116, 117, 115, 45, 99, 111, 100, 101, 1, 121, 1, 64, 2, 4, 115, 101, 108, 102, 248, 0,
    11, 115, 116, 97, 116, 117, 115, 45, 99, 111, 100, 101, 42, 0, 210, 0, 4, 0, 41, 91, 109, 101,
    116, 104, 111, 100, 93, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 115, 112, 111,
    110, 115, 101, 46, 115, 101, 116, 45, 115, 116, 97, 116, 117, 115, 45, 99, 111, 100, 101, 1,
    122, 1, 64, 1, 4, 115, 101, 108, 102, 248, 0, 0, 198, 0, 4, 0, 33, 91, 109, 101, 116, 104, 111,
    100, 93, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101,
    46, 104, 101, 97, 100, 101, 114, 115, 1, 123, 1, 64, 1, 4, 115, 101, 108, 102, 248, 0, 0, 207,
    0, 4, 0, 30, 91, 109, 101, 116, 104, 111, 100, 93, 111, 117, 116, 103, 111, 105, 110, 103, 45,
    114, 101, 115, 112, 111, 110, 115, 101, 46, 98, 111, 100, 121, 1, 124, 1, 104, 47, 1, 105, 5,
    1, 106, 1, 254, 0, 0, 1, 64, 1, 4, 115, 101, 108, 102, 253, 0, 0, 255, 0, 4, 0, 27, 91, 109,
    101, 116, 104, 111, 100, 93, 111, 117, 116, 103, 111, 105, 110, 103, 45, 98, 111, 100, 121, 46,
    119, 114, 105, 116, 101, 1, 128, 1, 1, 106, 0, 1, 27, 1, 64, 2, 4, 116, 104, 105, 115, 206, 0,
    8, 116, 114, 97, 105, 108, 101, 114, 115, 242, 0, 0, 129, 1, 4, 0, 28, 91, 115, 116, 97, 116,
    105, 99, 93, 111, 117, 116, 103, 111, 105, 110, 103, 45, 98, 111, 100, 121, 46, 102, 105, 110,
    105, 115, 104, 1, 130, 1, 1, 104, 48, 1, 64, 1, 4, 115, 101, 108, 102, 131, 1, 0, 239, 0, 4, 0,
    42, 91, 109, 101, 116, 104, 111, 100, 93, 102, 117, 116, 117, 114, 101, 45, 105, 110, 99, 111,
    109, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 46, 115, 117, 98, 115, 99, 114,
    105, 98, 101, 1, 132, 1, 1, 105, 43, 1, 106, 1, 133, 1, 1, 27, 1, 106, 1, 134, 1, 0, 1, 107,
    135, 1, 1, 64, 1, 4, 115, 101, 108, 102, 131, 1, 0, 136, 1, 4, 0, 36, 91, 109, 101, 116, 104,
    111, 100, 93, 102, 117, 116, 117, 114, 101, 45, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114,
    101, 115, 112, 111, 110, 115, 101, 46, 103, 101, 116, 1, 137, 1, 1, 104, 7, 1, 107, 27, 1, 64,
    1, 3, 101, 114, 114, 138, 1, 0, 139, 1, 4, 0, 15, 104, 116, 116, 112, 45, 101, 114, 114, 111,
    114, 45, 99, 111, 100, 101, 1, 140, 1, 3, 1, 35, 119, 97, 115, 105, 58, 104, 116, 116, 112, 47,
    116, 121, 112, 101, 115, 64, 48, 46, 50, 46, 48, 45, 114, 99, 45, 50, 48, 50, 51, 45, 49, 50,
    45, 48, 53, 5, 9, 2, 3, 0, 4, 16, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 113,
    117, 101, 115, 116, 2, 3, 0, 4, 15, 114, 101, 113, 117, 101, 115, 116, 45, 111, 112, 116, 105,
    111, 110, 115, 2, 3, 0, 4, 24, 102, 117, 116, 117, 114, 101, 45, 105, 110, 99, 111, 109, 105,
    110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 2, 3, 0, 4, 10, 101, 114, 114, 111, 114,
    45, 99, 111, 100, 101, 1, 66, 15, 2, 3, 2, 1, 10, 4, 0, 16, 111, 117, 116, 103, 111, 105, 110,
    103, 45, 114, 101, 113, 117, 101, 115, 116, 3, 0, 0, 2, 3, 2, 1, 11, 4, 0, 15, 114, 101, 113,
    117, 101, 115, 116, 45, 111, 112, 116, 105, 111, 110, 115, 3, 0, 2, 2, 3, 2, 1, 12, 4, 0, 24,
    102, 117, 116, 117, 114, 101, 45, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 115,
    112, 111, 110, 115, 101, 3, 0, 4, 2, 3, 2, 1, 13, 4, 0, 10, 101, 114, 114, 111, 114, 45, 99,
    111, 100, 101, 3, 0, 6, 1, 105, 1, 1, 105, 3, 1, 107, 9, 1, 105, 5, 1, 106, 1, 11, 1, 7, 1, 64,
    2, 7, 114, 101, 113, 117, 101, 115, 116, 8, 7, 111, 112, 116, 105, 111, 110, 115, 10, 0, 12, 4,
    0, 6, 104, 97, 110, 100, 108, 101, 1, 13, 3, 1, 46, 119, 97, 115, 105, 58, 104, 116, 116, 112,
    47, 111, 117, 116, 103, 111, 105, 110, 103, 45, 104, 97, 110, 100, 108, 101, 114, 64, 48, 46,
    50, 46, 48, 45, 114, 99, 45, 50, 48, 50, 51, 45, 49, 50, 45, 48, 53, 5, 14, 1, 66, 28, 1, 113,
    3, 4, 116, 105, 109, 101, 1, 119, 0, 11, 116, 101, 109, 112, 101, 114, 97, 116, 117, 114, 101,
    1, 117, 0, 10, 119, 105, 110, 100, 45, 115, 112, 101, 101, 100, 1, 117, 0, 4, 0, 6, 117, 112,
    100, 97, 116, 101, 3, 0, 0, 1, 114, 3, 2, 105, 100, 119, 9, 116, 105, 109, 101, 115, 116, 97,
    109, 112, 119, 5, 105, 110, 110, 101, 114, 1, 4, 0, 5, 101, 118, 101, 110, 116, 3, 0, 2, 1,
    113, 7, 6, 115, 101, 99, 111, 110, 100, 0, 0, 6, 109, 105, 110, 117, 116, 101, 0, 0, 4, 104,
    111, 117, 114, 0, 0, 3, 100, 97, 121, 0, 0, 4, 119, 101, 101, 107, 0, 0, 5, 109, 111, 110, 116,
    104, 0, 0, 4, 121, 101, 97, 114, 0, 0, 4, 0, 9, 116, 105, 109, 101, 45, 117, 110, 105, 116, 3,
    0, 4, 1, 114, 2, 6, 97, 109, 111, 117, 110, 116, 119, 4, 117, 110, 105, 116, 5, 4, 0, 5, 101,
    118, 101, 114, 121, 3, 0, 6, 1, 113, 2, 5, 101, 118, 101, 114, 121, 1, 7, 0, 2, 97, 116, 1,
    119, 0, 4, 0, 17, 116, 105, 109, 101, 45, 115, 117, 98, 115, 99, 114, 105, 112, 116, 105, 111,
    110, 3, 0, 8, 1, 113, 1, 4, 116, 105, 109, 101, 1, 9, 0, 4, 0, 12, 115, 117, 98, 115, 99, 114,
    105, 112, 116, 105, 111, 110, 3, 0, 10, 4, 0, 6, 114, 117, 110, 110, 101, 114, 3, 1, 1, 105,
    12, 1, 64, 0, 0, 13, 4, 0, 19, 91, 99, 111, 110, 115, 116, 114, 117, 99, 116, 111, 114, 93,
    114, 117, 110, 110, 101, 114, 1, 14, 1, 104, 12, 1, 112, 11, 1, 106, 1, 16, 1, 121, 1, 64, 1,
    4, 115, 101, 108, 102, 15, 0, 17, 4, 0, 24, 91, 109, 101, 116, 104, 111, 100, 93, 114, 117,
    110, 110, 101, 114, 46, 115, 117, 98, 115, 99, 114, 105, 98, 101, 1, 18, 1, 112, 3, 1, 106, 1,
    127, 1, 121, 1, 64, 2, 4, 115, 101, 108, 102, 15, 6, 101, 118, 101, 110, 116, 115, 19, 0, 20,
    4, 0, 21, 91, 109, 101, 116, 104, 111, 100, 93, 114, 117, 110, 110, 101, 114, 46, 117, 112,
    100, 97, 116, 101, 1, 21, 1, 107, 115, 1, 64, 0, 0, 22, 4, 0, 22, 103, 101, 110, 101, 114, 97,
    116, 101, 45, 99, 111, 110, 102, 105, 103, 45, 115, 99, 104, 101, 109, 97, 1, 23, 3, 1, 23,
    108, 105, 116, 101, 104, 111, 117, 115, 101, 58, 112, 108, 117, 103, 105, 110, 47, 112, 108,
    117, 103, 105, 110, 5, 15, 2, 3, 0, 6, 5, 101, 118, 101, 110, 116, 3, 0, 5, 101, 118, 101, 110,
    116, 3, 0, 16, 2, 3, 0, 6, 12, 115, 117, 98, 115, 99, 114, 105, 112, 116, 105, 111, 110, 3, 0,
    12, 115, 117, 98, 115, 99, 114, 105, 112, 116, 105, 111, 110, 3, 0, 18, 1, 64, 1, 5, 101, 118,
    101, 110, 116, 17, 1, 0, 3, 0, 6, 117, 112, 100, 97, 116, 101, 1, 20, 1, 66, 28, 1, 113, 3, 4,
    116, 105, 109, 101, 1, 119, 0, 11, 116, 101, 109, 112, 101, 114, 97, 116, 117, 114, 101, 1,
    117, 0, 10, 119, 105, 110, 100, 45, 115, 112, 101, 101, 100, 1, 117, 0, 4, 0, 6, 117, 112, 100,
    97, 116, 101, 3, 0, 0, 1, 114, 3, 2, 105, 100, 119, 9, 116, 105, 109, 101, 115, 116, 97, 109,
    112, 119, 5, 105, 110, 110, 101, 114, 1, 4, 0, 5, 101, 118, 101, 110, 116, 3, 0, 2, 1, 113, 7,
    6, 115, 101, 99, 111, 110, 100, 0, 0, 6, 109, 105, 110, 117, 116, 101, 0, 0, 4, 104, 111, 117,
    114, 0, 0, 3, 100, 97, 121, 0, 0, 4, 119, 101, 101, 107, 0, 0, 5, 109, 111, 110, 116, 104, 0,
    0, 4, 121, 101, 97, 114, 0, 0, 4, 0, 9, 116, 105, 109, 101, 45, 117, 110, 105, 116, 3, 0, 4, 1,
    114, 2, 6, 97, 109, 111, 117, 110, 116, 119, 4, 117, 110, 105, 116, 5, 4, 0, 5, 101, 118, 101,
    114, 121, 3, 0, 6, 1, 113, 2, 5, 101, 118, 101, 114, 121, 1, 7, 0, 2, 97, 116, 1, 119, 0, 4, 0,
    17, 116, 105, 109, 101, 45, 115, 117, 98, 115, 99, 114, 105, 112, 116, 105, 111, 110, 3, 0, 8,
    1, 113, 1, 4, 116, 105, 109, 101, 1, 9, 0, 4, 0, 12, 115, 117, 98, 115, 99, 114, 105, 112, 116,
    105, 111, 110, 3, 0, 10, 4, 0, 6, 114, 117, 110, 110, 101, 114, 3, 1, 1, 105, 12, 1, 64, 0, 0,
    13, 4, 0, 19, 91, 99, 111, 110, 115, 116, 114, 117, 99, 116, 111, 114, 93, 114, 117, 110, 110,
    101, 114, 1, 14, 1, 104, 12, 1, 112, 11, 1, 106, 1, 16, 1, 121, 1, 64, 1, 4, 115, 101, 108,
    102, 15, 0, 17, 4, 0, 24, 91, 109, 101, 116, 104, 111, 100, 93, 114, 117, 110, 110, 101, 114,
    46, 115, 117, 98, 115, 99, 114, 105, 98, 101, 1, 18, 1, 112, 3, 1, 106, 1, 127, 1, 121, 1, 64,
    2, 4, 115, 101, 108, 102, 15, 6, 101, 118, 101, 110, 116, 115, 19, 0, 20, 4, 0, 21, 91, 109,
    101, 116, 104, 111, 100, 93, 114, 117, 110, 110, 101, 114, 46, 117, 112, 100, 97, 116, 101, 1,
    21, 1, 107, 115, 1, 64, 0, 0, 22, 4, 0, 22, 103, 101, 110, 101, 114, 97, 116, 101, 45, 99, 111,
    110, 102, 105, 103, 45, 115, 99, 104, 101, 109, 97, 1, 23, 4, 1, 23, 108, 105, 116, 101, 104,
    111, 117, 115, 101, 58, 112, 108, 117, 103, 105, 110, 47, 112, 108, 117, 103, 105, 110, 5, 21,
    4, 1, 28, 108, 105, 116, 101, 104, 111, 117, 115, 101, 58, 112, 108, 117, 103, 105, 110, 47,
    112, 108, 117, 103, 105, 110, 45, 104, 111, 115, 116, 4, 0, 11, 17, 1, 0, 11, 112, 108, 117,
    103, 105, 110, 45, 104, 111, 115, 116, 3, 2, 0, 0, 16, 12, 112, 97, 99, 107, 97, 103, 101, 45,
    100, 111, 99, 115, 0, 123, 125, 0, 70, 9, 112, 114, 111, 100, 117, 99, 101, 114, 115, 1, 12,
    112, 114, 111, 99, 101, 115, 115, 101, 100, 45, 98, 121, 2, 13, 119, 105, 116, 45, 99, 111,
    109, 112, 111, 110, 101, 110, 116, 6, 48, 46, 49, 56, 46, 50, 16, 119, 105, 116, 45, 98, 105,
    110, 100, 103, 101, 110, 45, 114, 117, 115, 116, 6, 48, 46, 49, 54, 46, 48,
];
#[inline(never)]
#[doc(hidden)]
#[cfg(target_arch = "wasm32")]
pub fn __link_section() {}
const _ : & str = "package wasi:io@0.2.0-rc-2023-11-10;\n\n\ninterface error {\n    /// A resource which represents some error information.\n    ///\n    /// The only method provided by this resource is `to-debug-string`,\n    /// which provides some human-readable information about the error.\n    ///\n    /// In the `wasi:io` package, this resource is returned through the\n    /// `wasi:io/streams/stream-error` type.\n    ///\n    /// To provide more specific error information, other interfaces may\n    /// provide functions to further \"downcast\" this error into more specific\n    /// error information. For example, `error`s returned in streams derived\n    /// from filesystem types to be described using the filesystem\'s own\n    /// error-code type, using the function\n    /// `wasi:filesystem/types/filesystem-error-code`, which takes a parameter\n    /// `borrow<error>` and returns\n    /// `option<wasi:filesystem/types/error-code>`.\n    ///\n    /// The set of functions which can \"downcast\" an `error` into a more\n    /// concrete type is open.\n    resource error {\n        /// Returns a string that is suitable to assist humans in debugging\n        /// this error.\n        ///\n        /// WARNING: The returned string should not be consumed mechanically!\n        /// It may change across platforms, hosts, or other implementation\n        /// details. Parsing this string is a major platform-compatibility\n        /// hazard.\n        to-debug-string: func() -> string;\n    }\n}\n" ;
const _ : & str = "package wasi:io@0.2.0-rc-2023-11-10;\n\n/// A poll API intended to let users wait for I/O events on multiple handles\n/// at once.\ninterface poll {\n    /// `pollable` represents a single I/O event which may be ready, or not.\n    resource pollable {\n\n      /// Return the readiness of a pollable. This function never blocks.\n      ///\n      /// Returns `true` when the pollable is ready, and `false` otherwise.\n      ready: func() -> bool;\n\n      /// `block` returns immediately if the pollable is ready, and otherwise\n      /// blocks until ready.\n      ///\n      /// This function is equivalent to calling `poll.poll` on a list\n      /// containing only this pollable.\n      block: func();\n    }\n\n    /// Poll for completion on a set of pollables.\n    ///\n    /// This function takes a list of pollables, which identify I/O sources of\n    /// interest, and waits until one or more of the events is ready for I/O.\n    ///\n    /// The result `list<u32>` contains one or more indices of handles in the\n    /// argument list that is ready for I/O.\n    ///\n    /// If the list contains more elements than can be indexed with a `u32`\n    /// value, this function traps.\n    ///\n    /// A timeout can be implemented by adding a pollable from the\n    /// wasi-clocks API to the list.\n    ///\n    /// This function does not return a `result`; polling in itself does not\n    /// do any I/O so it doesn\'t fail. If any of the I/O sources identified by\n    /// the pollables has an error, it is indicated by marking the source as\n    /// being reaedy for I/O.\n    poll: func(in: list<borrow<pollable>>) -> list<u32>;\n}\n" ;
const _ : & str = "package wasi:io@0.2.0-rc-2023-11-10;\n\n/// WASI I/O is an I/O abstraction API which is currently focused on providing\n/// stream types.\n///\n/// In the future, the component model is expected to add built-in stream types;\n/// when it does, they are expected to subsume this API.\ninterface streams {\n    use error.{error};\n    use poll.{pollable};\n\n    /// An error for input-stream and output-stream operations.\n    variant stream-error {\n        /// The last operation (a write or flush) failed before completion.\n        ///\n        /// More information is available in the `error` payload.\n        last-operation-failed(error),\n        /// The stream is closed: no more input will be accepted by the\n        /// stream. A closed output-stream will return this error on all\n        /// future operations.\n        closed\n    }\n\n    /// An input bytestream.\n    ///\n    /// `input-stream`s are *non-blocking* to the extent practical on underlying\n    /// platforms. I/O operations always return promptly; if fewer bytes are\n    /// promptly available than requested, they return the number of bytes promptly\n    /// available, which could even be zero. To wait for data to be available,\n    /// use the `subscribe` function to obtain a `pollable` which can be polled\n    /// for using `wasi:io/poll`.\n    resource input-stream {\n        /// Perform a non-blocking read from the stream.\n        ///\n        /// This function returns a list of bytes containing the read data,\n        /// when successful. The returned list will contain up to `len` bytes;\n        /// it may return fewer than requested, but not more. The list is\n        /// empty when no bytes are available for reading at this time. The\n        /// pollable given by `subscribe` will be ready when more bytes are\n        /// available.\n        ///\n        /// This function fails with a `stream-error` when the operation\n        /// encounters an error, giving `last-operation-failed`, or when the\n        /// stream is closed, giving `closed`.\n        ///\n        /// When the caller gives a `len` of 0, it represents a request to\n        /// read 0 bytes. If the stream is still open, this call should\n        /// succeed and return an empty list, or otherwise fail with `closed`.\n        ///\n        /// The `len` parameter is a `u64`, which could represent a list of u8 which\n        /// is not possible to allocate in wasm32, or not desirable to allocate as\n        /// as a return value by the callee. The callee may return a list of bytes\n        /// less than `len` in size while more bytes are available for reading.\n        read: func(\n            /// The maximum number of bytes to read\n            len: u64\n        ) -> result<list<u8>, stream-error>;\n\n        /// Read bytes from a stream, after blocking until at least one byte can\n        /// be read. Except for blocking, behavior is identical to `read`.\n        blocking-read: func(\n            /// The maximum number of bytes to read\n            len: u64\n        ) -> result<list<u8>, stream-error>;\n\n        /// Skip bytes from a stream. Returns number of bytes skipped.\n        ///\n        /// Behaves identical to `read`, except instead of returning a list\n        /// of bytes, returns the number of bytes consumed from the stream.\n        skip: func(\n            /// The maximum number of bytes to skip.\n            len: u64,\n        ) -> result<u64, stream-error>;\n\n        /// Skip bytes from a stream, after blocking until at least one byte\n        /// can be skipped. Except for blocking behavior, identical to `skip`.\n        blocking-skip: func(\n            /// The maximum number of bytes to skip.\n            len: u64,\n        ) -> result<u64, stream-error>;\n\n        /// Create a `pollable` which will resolve once either the specified stream\n        /// has bytes available to read or the other end of the stream has been\n        /// closed.\n        /// The created `pollable` is a child resource of the `input-stream`.\n        /// Implementations may trap if the `input-stream` is dropped before\n        /// all derived `pollable`s created with this function are dropped.\n        subscribe: func() -> pollable;\n    }\n\n\n    /// An output bytestream.\n    ///\n    /// `output-stream`s are *non-blocking* to the extent practical on\n    /// underlying platforms. Except where specified otherwise, I/O operations also\n    /// always return promptly, after the number of bytes that can be written\n    /// promptly, which could even be zero. To wait for the stream to be ready to\n    /// accept data, the `subscribe` function to obtain a `pollable` which can be\n    /// polled for using `wasi:io/poll`.\n    resource output-stream {\n        /// Check readiness for writing. This function never blocks.\n        ///\n        /// Returns the number of bytes permitted for the next call to `write`,\n        /// or an error. Calling `write` with more bytes than this function has\n        /// permitted will trap.\n        ///\n        /// When this function returns 0 bytes, the `subscribe` pollable will\n        /// become ready when this function will report at least 1 byte, or an\n        /// error.\n        check-write: func() -> result<u64, stream-error>;\n\n        /// Perform a write. This function never blocks.\n        ///\n        /// Precondition: check-write gave permit of Ok(n) and contents has a\n        /// length of less than or equal to n. Otherwise, this function will trap.\n        ///\n        /// returns Err(closed) without writing if the stream has closed since\n        /// the last call to check-write provided a permit.\n        write: func(\n            contents: list<u8>\n        ) -> result<_, stream-error>;\n\n        /// Perform a write of up to 4096 bytes, and then flush the stream. Block\n        /// until all of these operations are complete, or an error occurs.\n        ///\n        /// This is a convenience wrapper around the use of `check-write`,\n        /// `subscribe`, `write`, and `flush`, and is implemented with the\n        /// following pseudo-code:\n        ///\n        /// ```text\n        /// let pollable = this.subscribe();\n        /// while !contents.is_empty() {\n        ///     // Wait for the stream to become writable\n        ///     pollable.block();\n        ///     let Ok(n) = this.check-write(); // eliding error handling\n        ///     let len = min(n, contents.len());\n        ///     let (chunk, rest) = contents.split_at(len);\n        ///     this.write(chunk  );            // eliding error handling\n        ///     contents = rest;\n        /// }\n        /// this.flush();\n        /// // Wait for completion of `flush`\n        /// pollable.block();\n        /// // Check for any errors that arose during `flush`\n        /// let _ = this.check-write();         // eliding error handling\n        /// ```\n        blocking-write-and-flush: func(\n            contents: list<u8>\n        ) -> result<_, stream-error>;\n\n        /// Request to flush buffered output. This function never blocks.\n        ///\n        /// This tells the output-stream that the caller intends any buffered\n        /// output to be flushed. the output which is expected to be flushed\n        /// is all that has been passed to `write` prior to this call.\n        ///\n        /// Upon calling this function, the `output-stream` will not accept any\n        /// writes (`check-write` will return `ok(0)`) until the flush has\n        /// completed. The `subscribe` pollable will become ready when the\n        /// flush has completed and the stream can accept more writes.\n        flush: func() -> result<_, stream-error>;\n\n        /// Request to flush buffered output, and block until flush completes\n        /// and stream is ready for writing again.\n        blocking-flush: func() -> result<_, stream-error>;\n\n        /// Create a `pollable` which will resolve once the output-stream\n        /// is ready for more writing, or an error has occured. When this\n        /// pollable is ready, `check-write` will return `ok(n)` with n>0, or an\n        /// error.\n        ///\n        /// If the stream is closed, this pollable is always ready immediately.\n        ///\n        /// The created `pollable` is a child resource of the `output-stream`.\n        /// Implementations may trap if the `output-stream` is dropped before\n        /// all derived `pollable`s created with this function are dropped.\n        subscribe: func() -> pollable;\n\n        /// Write zeroes to a stream.\n        ///\n        /// This should be used precisely like `write` with the exact same\n        /// preconditions (must use check-write first), but instead of\n        /// passing a list of bytes, you simply pass the number of zero-bytes\n        /// that should be written.\n        write-zeroes: func(\n            /// The number of zero-bytes to write\n            len: u64\n        ) -> result<_, stream-error>;\n\n        /// Perform a write of up to 4096 zeroes, and then flush the stream.\n        /// Block until all of these operations are complete, or an error\n        /// occurs.\n        ///\n        /// This is a convenience wrapper around the use of `check-write`,\n        /// `subscribe`, `write-zeroes`, and `flush`, and is implemented with\n        /// the following pseudo-code:\n        ///\n        /// ```text\n        /// let pollable = this.subscribe();\n        /// while num_zeroes != 0 {\n        ///     // Wait for the stream to become writable\n        ///     pollable.block();\n        ///     let Ok(n) = this.check-write(); // eliding error handling\n        ///     let len = min(n, num_zeroes);\n        ///     this.write-zeroes(len);         // eliding error handling\n        ///     num_zeroes -= len;\n        /// }\n        /// this.flush();\n        /// // Wait for completion of `flush`\n        /// pollable.block();\n        /// // Check for any errors that arose during `flush`\n        /// let _ = this.check-write();         // eliding error handling\n        /// ```\n        blocking-write-zeroes-and-flush: func(\n            /// The number of zero-bytes to write\n            len: u64\n        ) -> result<_, stream-error>;\n\n        /// Read from one stream and write to another.\n        ///\n        /// The behavior of splice is equivelant to:\n        /// 1. calling `check-write` on the `output-stream`\n        /// 2. calling `read` on the `input-stream` with the smaller of the\n        /// `check-write` permitted length and the `len` provided to `splice`\n        /// 3. calling `write` on the `output-stream` with that read data.\n        ///\n        /// Any error reported by the call to `check-write`, `read`, or\n        /// `write` ends the splice and reports that error.\n        ///\n        /// This function returns the number of bytes transferred; it may be less\n        /// than `len`.\n        splice: func(\n            /// The stream to read from\n            src: borrow<input-stream>,\n            /// The number of bytes to splice\n            len: u64,\n        ) -> result<u64, stream-error>;\n\n        /// Read from one stream and write to another, with blocking.\n        ///\n        /// This is similar to `splice`, except that it blocks until the\n        /// `output-stream` is ready for writing, and the `input-stream`\n        /// is ready for reading, before performing the `splice`.\n        blocking-splice: func(\n            /// The stream to read from\n            src: borrow<input-stream>,\n            /// The number of bytes to splice\n            len: u64,\n        ) -> result<u64, stream-error>;\n    }\n}\n" ;
const _ : & str = "package wasi:io@0.2.0-rc-2023-11-10;\n\nworld imports {\n    import streams;\n    import poll;\n}\n" ;
const _ : & str = "package wasi:clocks@0.2.0-rc-2023-11-10;\n/// WASI Monotonic Clock is a clock API intended to let users measure elapsed\n/// time.\n///\n/// It is intended to be portable at least between Unix-family platforms and\n/// Windows.\n///\n/// A monotonic clock is a clock which has an unspecified initial value, and\n/// successive reads of the clock will produce non-decreasing values.\n///\n/// It is intended for measuring elapsed time.\ninterface monotonic-clock {\n    use wasi:io/poll@0.2.0-rc-2023-11-10.{pollable};\n\n    /// An instant in time, in nanoseconds. An instant is relative to an\n    /// unspecified initial value, and can only be compared to instances from\n    /// the same monotonic-clock.\n    type instant = u64;\n\n    /// A duration of time, in nanoseconds.\n    type duration = u64;\n\n    /// Read the current value of the clock.\n    ///\n    /// The clock is monotonic, therefore calling this function repeatedly will\n    /// produce a sequence of non-decreasing values.\n    now: func() -> instant;\n\n    /// Query the resolution of the clock. Returns the duration of time\n    /// corresponding to a clock tick.\n    resolution: func() -> duration;\n\n    /// Create a `pollable` which will resolve once the specified instant\n    /// occured.\n    subscribe-instant: func(\n        when: instant,\n    ) -> pollable;\n\n    /// Create a `pollable` which will resolve once the given duration has\n    /// elapsed, starting at the time at which this function was called.\n    /// occured.\n    subscribe-duration: func(\n        when: duration,\n    ) -> pollable;\n}\n" ;
const _ : & str = "package wasi:clocks@0.2.0-rc-2023-11-10;\n/// WASI Wall Clock is a clock API intended to let users query the current\n/// time. The name \"wall\" makes an analogy to a \"clock on the wall\", which\n/// is not necessarily monotonic as it may be reset.\n///\n/// It is intended to be portable at least between Unix-family platforms and\n/// Windows.\n///\n/// A wall clock is a clock which measures the date and time according to\n/// some external reference.\n///\n/// External references may be reset, so this clock is not necessarily\n/// monotonic, making it unsuitable for measuring elapsed time.\n///\n/// It is intended for reporting the current date and time for humans.\ninterface wall-clock {\n    /// A time and date in seconds plus nanoseconds.\n    record datetime {\n        seconds: u64,\n        nanoseconds: u32,\n    }\n\n    /// Read the current value of the clock.\n    ///\n    /// This clock is not monotonic, therefore calling this function repeatedly\n    /// will not necessarily produce a sequence of non-decreasing values.\n    ///\n    /// The returned timestamps represent the number of seconds since\n    /// 1970-01-01T00:00:00Z, also known as [POSIX\'s Seconds Since the Epoch],\n    /// also known as [Unix Time].\n    ///\n    /// The nanoseconds field of the output is always less than 1000000000.\n    ///\n    /// [POSIX\'s Seconds Since the Epoch]: https://pubs.opengroup.org/onlinepubs/9699919799/xrat/V4_xbd_chap04.html#tag_21_04_16\n    /// [Unix Time]: https://en.wikipedia.org/wiki/Unix_time\n    now: func() -> datetime;\n\n    /// Query the resolution of the clock.\n    ///\n    /// The nanoseconds field of the output is always less than 1000000000.\n    resolution: func() -> datetime;\n}\n" ;
const _ : & str = "package wasi:clocks@0.2.0-rc-2023-11-10;\n\nworld imports {\n    import monotonic-clock;\n    import wall-clock;\n}\n" ;
const _ : & str = "package wasi:filesystem@0.2.0-rc-2023-11-10;\n\ninterface preopens {\n    use types.{descriptor};\n\n    /// Return the set of preopened directories, and their path.\n    get-directories: func() -> list<tuple<descriptor, string>>;\n}\n" ;
const _ : & str = "package wasi:filesystem@0.2.0-rc-2023-11-10;\n/// WASI filesystem is a filesystem API primarily intended to let users run WASI\n/// programs that access their files on their existing filesystems, without\n/// significant overhead.\n///\n/// It is intended to be roughly portable between Unix-family platforms and\n/// Windows, though it does not hide many of the major differences.\n///\n/// Paths are passed as interface-type `string`s, meaning they must consist of\n/// a sequence of Unicode Scalar Values (USVs). Some filesystems may contain\n/// paths which are not accessible by this API.\n///\n/// The directory separator in WASI is always the forward-slash (`/`).\n///\n/// All paths in WASI are relative paths, and are interpreted relative to a\n/// `descriptor` referring to a base directory. If a `path` argument to any WASI\n/// function starts with `/`, or if any step of resolving a `path`, including\n/// `..` and symbolic link steps, reaches a directory outside of the base\n/// directory, or reaches a symlink to an absolute or rooted path in the\n/// underlying filesystem, the function fails with `error-code::not-permitted`.\n///\n/// For more information about WASI path resolution and sandboxing, see\n/// [WASI filesystem path resolution].\n///\n/// [WASI filesystem path resolution]: https://github.com/WebAssembly/wasi-filesystem/blob/main/path-resolution.md\ninterface types {\n    use wasi:io/streams@0.2.0-rc-2023-11-10.{input-stream, output-stream, error};\n    use wasi:clocks/wall-clock@0.2.0-rc-2023-11-10.{datetime};\n\n    /// File size or length of a region within a file.\n    type filesize = u64;\n\n    /// The type of a filesystem object referenced by a descriptor.\n    ///\n    /// Note: This was called `filetype` in earlier versions of WASI.\n    enum descriptor-type {\n        /// The type of the descriptor or file is unknown or is different from\n        /// any of the other types specified.\n        unknown,\n        /// The descriptor refers to a block device inode.\n        block-device,\n        /// The descriptor refers to a character device inode.\n        character-device,\n        /// The descriptor refers to a directory inode.\n        directory,\n        /// The descriptor refers to a named pipe.\n        fifo,\n        /// The file refers to a symbolic link inode.\n        symbolic-link,\n        /// The descriptor refers to a regular file inode.\n        regular-file,\n        /// The descriptor refers to a socket.\n        socket,\n    }\n\n    /// Descriptor flags.\n    ///\n    /// Note: This was called `fdflags` in earlier versions of WASI.\n    flags descriptor-flags {\n        /// Read mode: Data can be read.\n        read,\n        /// Write mode: Data can be written to.\n        write,\n        /// Request that writes be performed according to synchronized I/O file\n        /// integrity completion. The data stored in the file and the file\'s\n        /// metadata are synchronized. This is similar to `O_SYNC` in POSIX.\n        ///\n        /// The precise semantics of this operation have not yet been defined for\n        /// WASI. At this time, it should be interpreted as a request, and not a\n        /// requirement.\n        file-integrity-sync,\n        /// Request that writes be performed according to synchronized I/O data\n        /// integrity completion. Only the data stored in the file is\n        /// synchronized. This is similar to `O_DSYNC` in POSIX.\n        ///\n        /// The precise semantics of this operation have not yet been defined for\n        /// WASI. At this time, it should be interpreted as a request, and not a\n        /// requirement.\n        data-integrity-sync,\n        /// Requests that reads be performed at the same level of integrety\n        /// requested for writes. This is similar to `O_RSYNC` in POSIX.\n        ///\n        /// The precise semantics of this operation have not yet been defined for\n        /// WASI. At this time, it should be interpreted as a request, and not a\n        /// requirement.\n        requested-write-sync,\n        /// Mutating directories mode: Directory contents may be mutated.\n        ///\n        /// When this flag is unset on a descriptor, operations using the\n        /// descriptor which would create, rename, delete, modify the data or\n        /// metadata of filesystem objects, or obtain another handle which\n        /// would permit any of those, shall fail with `error-code::read-only` if\n        /// they would otherwise succeed.\n        ///\n        /// This may only be set on directories.\n        mutate-directory,\n    }\n\n    /// File attributes.\n    ///\n    /// Note: This was called `filestat` in earlier versions of WASI.\n    record descriptor-stat {\n        /// File type.\n        %type: descriptor-type,\n        /// Number of hard links to the file.\n        link-count: link-count,\n        /// For regular files, the file size in bytes. For symbolic links, the\n        /// length in bytes of the pathname contained in the symbolic link.\n        size: filesize,\n        /// Last data access timestamp.\n        ///\n        /// If the `option` is none, the platform doesn\'t maintain an access\n        /// timestamp for this file.\n        data-access-timestamp: option<datetime>,\n        /// Last data modification timestamp.\n        ///\n        /// If the `option` is none, the platform doesn\'t maintain a\n        /// modification timestamp for this file.\n        data-modification-timestamp: option<datetime>,\n        /// Last file status-change timestamp.\n        ///\n        /// If the `option` is none, the platform doesn\'t maintain a\n        /// status-change timestamp for this file.\n        status-change-timestamp: option<datetime>,\n    }\n\n    /// Flags determining the method of how paths are resolved.\n    flags path-flags {\n        /// As long as the resolved path corresponds to a symbolic link, it is\n        /// expanded.\n        symlink-follow,\n    }\n\n    /// Open flags used by `open-at`.\n    flags open-flags {\n        /// Create file if it does not exist, similar to `O_CREAT` in POSIX.\n        create,\n        /// Fail if not a directory, similar to `O_DIRECTORY` in POSIX.\n        directory,\n        /// Fail if file already exists, similar to `O_EXCL` in POSIX.\n        exclusive,\n        /// Truncate file to size 0, similar to `O_TRUNC` in POSIX.\n        truncate,\n    }\n\n    /// Number of hard links to an inode.\n    type link-count = u64;\n\n    /// When setting a timestamp, this gives the value to set it to.\n    variant new-timestamp {\n        /// Leave the timestamp set to its previous value.\n        no-change,\n        /// Set the timestamp to the current time of the system clock associated\n        /// with the filesystem.\n        now,\n        /// Set the timestamp to the given value.\n        timestamp(datetime),\n    }\n\n    /// A directory entry.\n    record directory-entry {\n        /// The type of the file referred to by this directory entry.\n        %type: descriptor-type,\n\n        /// The name of the object.\n        name: string,\n    }\n\n    /// Error codes returned by functions, similar to `errno` in POSIX.\n    /// Not all of these error codes are returned by the functions provided by this\n    /// API; some are used in higher-level library layers, and others are provided\n    /// merely for alignment with POSIX.\n    enum error-code {\n        /// Permission denied, similar to `EACCES` in POSIX.\n        access,\n        /// Resource unavailable, or operation would block, similar to `EAGAIN` and `EWOULDBLOCK` in POSIX.\n        would-block,\n        /// Connection already in progress, similar to `EALREADY` in POSIX.\n        already,\n        /// Bad descriptor, similar to `EBADF` in POSIX.\n        bad-descriptor,\n        /// Device or resource busy, similar to `EBUSY` in POSIX.\n        busy,\n        /// Resource deadlock would occur, similar to `EDEADLK` in POSIX.\n        deadlock,\n        /// Storage quota exceeded, similar to `EDQUOT` in POSIX.\n        quota,\n        /// File exists, similar to `EEXIST` in POSIX.\n        exist,\n        /// File too large, similar to `EFBIG` in POSIX.\n        file-too-large,\n        /// Illegal byte sequence, similar to `EILSEQ` in POSIX.\n        illegal-byte-sequence,\n        /// Operation in progress, similar to `EINPROGRESS` in POSIX.\n        in-progress,\n        /// Interrupted function, similar to `EINTR` in POSIX.\n        interrupted,\n        /// Invalid argument, similar to `EINVAL` in POSIX.\n        invalid,\n        /// I/O error, similar to `EIO` in POSIX.\n        io,\n        /// Is a directory, similar to `EISDIR` in POSIX.\n        is-directory,\n        /// Too many levels of symbolic links, similar to `ELOOP` in POSIX.\n        loop,\n        /// Too many links, similar to `EMLINK` in POSIX.\n        too-many-links,\n        /// Message too large, similar to `EMSGSIZE` in POSIX.\n        message-size,\n        /// Filename too long, similar to `ENAMETOOLONG` in POSIX.\n        name-too-long,\n        /// No such device, similar to `ENODEV` in POSIX.\n        no-device,\n        /// No such file or directory, similar to `ENOENT` in POSIX.\n        no-entry,\n        /// No locks available, similar to `ENOLCK` in POSIX.\n        no-lock,\n        /// Not enough space, similar to `ENOMEM` in POSIX.\n        insufficient-memory,\n        /// No space left on device, similar to `ENOSPC` in POSIX.\n        insufficient-space,\n        /// Not a directory or a symbolic link to a directory, similar to `ENOTDIR` in POSIX.\n        not-directory,\n        /// Directory not empty, similar to `ENOTEMPTY` in POSIX.\n        not-empty,\n        /// State not recoverable, similar to `ENOTRECOVERABLE` in POSIX.\n        not-recoverable,\n        /// Not supported, similar to `ENOTSUP` and `ENOSYS` in POSIX.\n        unsupported,\n        /// Inappropriate I/O control operation, similar to `ENOTTY` in POSIX.\n        no-tty,\n        /// No such device or address, similar to `ENXIO` in POSIX.\n        no-such-device,\n        /// Value too large to be stored in data type, similar to `EOVERFLOW` in POSIX.\n        overflow,\n        /// Operation not permitted, similar to `EPERM` in POSIX.\n        not-permitted,\n        /// Broken pipe, similar to `EPIPE` in POSIX.\n        pipe,\n        /// Read-only file system, similar to `EROFS` in POSIX.\n        read-only,\n        /// Invalid seek, similar to `ESPIPE` in POSIX.\n        invalid-seek,\n        /// Text file busy, similar to `ETXTBSY` in POSIX.\n        text-file-busy,\n        /// Cross-device link, similar to `EXDEV` in POSIX.\n        cross-device,\n    }\n\n    /// File or memory access pattern advisory information.\n    enum advice {\n        /// The application has no advice to give on its behavior with respect\n        /// to the specified data.\n        normal,\n        /// The application expects to access the specified data sequentially\n        /// from lower offsets to higher offsets.\n        sequential,\n        /// The application expects to access the specified data in a random\n        /// order.\n        random,\n        /// The application expects to access the specified data in the near\n        /// future.\n        will-need,\n        /// The application expects that it will not access the specified data\n        /// in the near future.\n        dont-need,\n        /// The application expects to access the specified data once and then\n        /// not reuse it thereafter.\n        no-reuse,\n    }\n\n    /// A 128-bit hash value, split into parts because wasm doesn\'t have a\n    /// 128-bit integer type.\n    record metadata-hash-value {\n       /// 64 bits of a 128-bit hash value.\n       lower: u64,\n       /// Another 64 bits of a 128-bit hash value.\n       upper: u64,\n    }\n\n    /// A descriptor is a reference to a filesystem object, which may be a file,\n    /// directory, named pipe, special file, or other object on which filesystem\n    /// calls may be made.\n    resource descriptor {\n        /// Return a stream for reading from a file, if available.\n        ///\n        /// May fail with an error-code describing why the file cannot be read.\n        ///\n        /// Multiple read, write, and append streams may be active on the same open\n        /// file and they do not interfere with each other.\n        ///\n        /// Note: This allows using `read-stream`, which is similar to `read` in POSIX.\n        read-via-stream: func(\n            /// The offset within the file at which to start reading.\n            offset: filesize,\n        ) -> result<input-stream, error-code>;\n\n        /// Return a stream for writing to a file, if available.\n        ///\n        /// May fail with an error-code describing why the file cannot be written.\n        ///\n        /// Note: This allows using `write-stream`, which is similar to `write` in\n        /// POSIX.\n        write-via-stream: func(\n            /// The offset within the file at which to start writing.\n            offset: filesize,\n        ) -> result<output-stream, error-code>;\n\n        /// Return a stream for appending to a file, if available.\n        ///\n        /// May fail with an error-code describing why the file cannot be appended.\n        ///\n        /// Note: This allows using `write-stream`, which is similar to `write` with\n        /// `O_APPEND` in in POSIX.\n        append-via-stream: func() -> result<output-stream, error-code>;\n\n        /// Provide file advisory information on a descriptor.\n        ///\n        /// This is similar to `posix_fadvise` in POSIX.\n        advise: func(\n            /// The offset within the file to which the advisory applies.\n            offset: filesize,\n            /// The length of the region to which the advisory applies.\n            length: filesize,\n            /// The advice.\n            advice: advice\n        ) -> result<_, error-code>;\n\n        /// Synchronize the data of a file to disk.\n        ///\n        /// This function succeeds with no effect if the file descriptor is not\n        /// opened for writing.\n        ///\n        /// Note: This is similar to `fdatasync` in POSIX.\n        sync-data: func() -> result<_, error-code>;\n\n        /// Get flags associated with a descriptor.\n        ///\n        /// Note: This returns similar flags to `fcntl(fd, F_GETFL)` in POSIX.\n        ///\n        /// Note: This returns the value that was the `fs_flags` value returned\n        /// from `fdstat_get` in earlier versions of WASI.\n        get-flags: func() -> result<descriptor-flags, error-code>;\n\n        /// Get the dynamic type of a descriptor.\n        ///\n        /// Note: This returns the same value as the `type` field of the `fd-stat`\n        /// returned by `stat`, `stat-at` and similar.\n        ///\n        /// Note: This returns similar flags to the `st_mode & S_IFMT` value provided\n        /// by `fstat` in POSIX.\n        ///\n        /// Note: This returns the value that was the `fs_filetype` value returned\n        /// from `fdstat_get` in earlier versions of WASI.\n        get-type: func() -> result<descriptor-type, error-code>;\n\n        /// Adjust the size of an open file. If this increases the file\'s size, the\n        /// extra bytes are filled with zeros.\n        ///\n        /// Note: This was called `fd_filestat_set_size` in earlier versions of WASI.\n        set-size: func(size: filesize) -> result<_, error-code>;\n\n        /// Adjust the timestamps of an open file or directory.\n        ///\n        /// Note: This is similar to `futimens` in POSIX.\n        ///\n        /// Note: This was called `fd_filestat_set_times` in earlier versions of WASI.\n        set-times: func(\n            /// The desired values of the data access timestamp.\n            data-access-timestamp: new-timestamp,\n            /// The desired values of the data modification timestamp.\n            data-modification-timestamp: new-timestamp,\n        ) -> result<_, error-code>;\n\n        /// Read from a descriptor, without using and updating the descriptor\'s offset.\n        ///\n        /// This function returns a list of bytes containing the data that was\n        /// read, along with a bool which, when true, indicates that the end of the\n        /// file was reached. The returned list will contain up to `length` bytes; it\n        /// may return fewer than requested, if the end of the file is reached or\n        /// if the I/O operation is interrupted.\n        ///\n        /// In the future, this may change to return a `stream<u8, error-code>`.\n        ///\n        /// Note: This is similar to `pread` in POSIX.\n        read: func(\n            /// The maximum number of bytes to read.\n            length: filesize,\n            /// The offset within the file at which to read.\n            offset: filesize,\n        ) -> result<tuple<list<u8>, bool>, error-code>;\n\n        /// Write to a descriptor, without using and updating the descriptor\'s offset.\n        ///\n        /// It is valid to write past the end of a file; the file is extended to the\n        /// extent of the write, with bytes between the previous end and the start of\n        /// the write set to zero.\n        ///\n        /// In the future, this may change to take a `stream<u8, error-code>`.\n        ///\n        /// Note: This is similar to `pwrite` in POSIX.\n        write: func(\n            /// Data to write\n            buffer: list<u8>,\n            /// The offset within the file at which to write.\n            offset: filesize,\n        ) -> result<filesize, error-code>;\n\n        /// Read directory entries from a directory.\n        ///\n        /// On filesystems where directories contain entries referring to themselves\n        /// and their parents, often named `.` and `..` respectively, these entries\n        /// are omitted.\n        ///\n        /// This always returns a new stream which starts at the beginning of the\n        /// directory. Multiple streams may be active on the same directory, and they\n        /// do not interfere with each other.\n        read-directory: func() -> result<directory-entry-stream, error-code>;\n\n        /// Synchronize the data and metadata of a file to disk.\n        ///\n        /// This function succeeds with no effect if the file descriptor is not\n        /// opened for writing.\n        ///\n        /// Note: This is similar to `fsync` in POSIX.\n        sync: func() -> result<_, error-code>;\n\n        /// Create a directory.\n        ///\n        /// Note: This is similar to `mkdirat` in POSIX.\n        create-directory-at: func(\n            /// The relative path at which to create the directory.\n            path: string,\n        ) -> result<_, error-code>;\n\n        /// Return the attributes of an open file or directory.\n        ///\n        /// Note: This is similar to `fstat` in POSIX, except that it does not return\n        /// device and inode information. For testing whether two descriptors refer to\n        /// the same underlying filesystem object, use `is-same-object`. To obtain\n        /// additional data that can be used do determine whether a file has been\n        /// modified, use `metadata-hash`.\n        ///\n        /// Note: This was called `fd_filestat_get` in earlier versions of WASI.\n        stat: func() -> result<descriptor-stat, error-code>;\n\n        /// Return the attributes of a file or directory.\n        ///\n        /// Note: This is similar to `fstatat` in POSIX, except that it does not\n        /// return device and inode information. See the `stat` description for a\n        /// discussion of alternatives.\n        ///\n        /// Note: This was called `path_filestat_get` in earlier versions of WASI.\n        stat-at: func(\n            /// Flags determining the method of how the path is resolved.\n            path-flags: path-flags,\n            /// The relative path of the file or directory to inspect.\n            path: string,\n        ) -> result<descriptor-stat, error-code>;\n\n        /// Adjust the timestamps of a file or directory.\n        ///\n        /// Note: This is similar to `utimensat` in POSIX.\n        ///\n        /// Note: This was called `path_filestat_set_times` in earlier versions of\n        /// WASI.\n        set-times-at: func(\n            /// Flags determining the method of how the path is resolved.\n            path-flags: path-flags,\n            /// The relative path of the file or directory to operate on.\n            path: string,\n            /// The desired values of the data access timestamp.\n            data-access-timestamp: new-timestamp,\n            /// The desired values of the data modification timestamp.\n            data-modification-timestamp: new-timestamp,\n        ) -> result<_, error-code>;\n\n        /// Create a hard link.\n        ///\n        /// Note: This is similar to `linkat` in POSIX.\n        link-at: func(\n            /// Flags determining the method of how the path is resolved.\n            old-path-flags: path-flags,\n            /// The relative source path from which to link.\n            old-path: string,\n            /// The base directory for `new-path`.\n            new-descriptor: borrow<descriptor>,\n            /// The relative destination path at which to create the hard link.\n            new-path: string,\n        ) -> result<_, error-code>;\n\n        /// Open a file or directory.\n        ///\n        /// The returned descriptor is not guaranteed to be the lowest-numbered\n        /// descriptor not currently open/ it is randomized to prevent applications\n        /// from depending on making assumptions about indexes, since this is\n        /// error-prone in multi-threaded contexts. The returned descriptor is\n        /// guaranteed to be less than 2**31.\n        ///\n        /// If `flags` contains `descriptor-flags::mutate-directory`, and the base\n        /// descriptor doesn\'t have `descriptor-flags::mutate-directory` set,\n        /// `open-at` fails with `error-code::read-only`.\n        ///\n        /// If `flags` contains `write` or `mutate-directory`, or `open-flags`\n        /// contains `truncate` or `create`, and the base descriptor doesn\'t have\n        /// `descriptor-flags::mutate-directory` set, `open-at` fails with\n        /// `error-code::read-only`.\n        ///\n        /// Note: This is similar to `openat` in POSIX.\n        open-at: func(\n            /// Flags determining the method of how the path is resolved.\n            path-flags: path-flags,\n            /// The relative path of the object to open.\n            path: string,\n            /// The method by which to open the file.\n            open-flags: open-flags,\n            /// Flags to use for the resulting descriptor.\n            %flags: descriptor-flags,\n        ) -> result<descriptor, error-code>;\n\n        /// Read the contents of a symbolic link.\n        ///\n        /// If the contents contain an absolute or rooted path in the underlying\n        /// filesystem, this function fails with `error-code::not-permitted`.\n        ///\n        /// Note: This is similar to `readlinkat` in POSIX.\n        readlink-at: func(\n            /// The relative path of the symbolic link from which to read.\n            path: string,\n        ) -> result<string, error-code>;\n\n        /// Remove a directory.\n        ///\n        /// Return `error-code::not-empty` if the directory is not empty.\n        ///\n        /// Note: This is similar to `unlinkat(fd, path, AT_REMOVEDIR)` in POSIX.\n        remove-directory-at: func(\n            /// The relative path to a directory to remove.\n            path: string,\n        ) -> result<_, error-code>;\n\n        /// Rename a filesystem object.\n        ///\n        /// Note: This is similar to `renameat` in POSIX.\n        rename-at: func(\n            /// The relative source path of the file or directory to rename.\n            old-path: string,\n            /// The base directory for `new-path`.\n            new-descriptor: borrow<descriptor>,\n            /// The relative destination path to which to rename the file or directory.\n            new-path: string,\n        ) -> result<_, error-code>;\n\n        /// Create a symbolic link (also known as a \"symlink\").\n        ///\n        /// If `old-path` starts with `/`, the function fails with\n        /// `error-code::not-permitted`.\n        ///\n        /// Note: This is similar to `symlinkat` in POSIX.\n        symlink-at: func(\n            /// The contents of the symbolic link.\n            old-path: string,\n            /// The relative destination path at which to create the symbolic link.\n            new-path: string,\n        ) -> result<_, error-code>;\n\n        /// Unlink a filesystem object that is not a directory.\n        ///\n        /// Return `error-code::is-directory` if the path refers to a directory.\n        /// Note: This is similar to `unlinkat(fd, path, 0)` in POSIX.\n        unlink-file-at: func(\n            /// The relative path to a file to unlink.\n            path: string,\n        ) -> result<_, error-code>;\n\n        /// Test whether two descriptors refer to the same filesystem object.\n        ///\n        /// In POSIX, this corresponds to testing whether the two descriptors have the\n        /// same device (`st_dev`) and inode (`st_ino` or `d_ino`) numbers.\n        /// wasi-filesystem does not expose device and inode numbers, so this function\n        /// may be used instead.\n        is-same-object: func(other: borrow<descriptor>) -> bool;\n\n        /// Return a hash of the metadata associated with a filesystem object referred\n        /// to by a descriptor.\n        ///\n        /// This returns a hash of the last-modification timestamp and file size, and\n        /// may also include the inode number, device number, birth timestamp, and\n        /// other metadata fields that may change when the file is modified or\n        /// replaced. It may also include a secret value chosen by the\n        /// implementation and not otherwise exposed.\n        ///\n        /// Implementations are encourated to provide the following properties:\n        ///\n        ///  - If the file is not modified or replaced, the computed hash value should\n        ///    usually not change.\n        ///  - If the object is modified or replaced, the computed hash value should\n        ///    usually change.\n        ///  - The inputs to the hash should not be easily computable from the\n        ///    computed hash.\n        ///\n        /// However, none of these is required.\n        metadata-hash: func() -> result<metadata-hash-value, error-code>;\n\n        /// Return a hash of the metadata associated with a filesystem object referred\n        /// to by a directory descriptor and a relative path.\n        ///\n        /// This performs the same hash computation as `metadata-hash`.\n        metadata-hash-at: func(\n            /// Flags determining the method of how the path is resolved.\n            path-flags: path-flags,\n            /// The relative path of the file or directory to inspect.\n            path: string,\n        ) -> result<metadata-hash-value, error-code>;\n    }\n\n    /// A stream of directory entries.\n    resource directory-entry-stream {\n        /// Read a single directory entry from a `directory-entry-stream`.\n        read-directory-entry: func() -> result<option<directory-entry>, error-code>;\n    }\n\n    /// Attempts to extract a filesystem-related `error-code` from the stream\n    /// `error` provided.\n    ///\n    /// Stream operations which return `stream-error::last-operation-failed`\n    /// have a payload with more information about the operation that failed.\n    /// This payload can be passed through to this function to see if there\'s\n    /// filesystem-related information about the error to return.\n    ///\n    /// Note that this function is fallible because not all stream-related\n    /// errors are filesystem-related errors.\n    filesystem-error-code: func(err: borrow<error>) -> option<error-code>;\n}\n" ;
const _ : & str = "package wasi:filesystem@0.2.0-rc-2023-11-10;\n\nworld imports {\n    import types;\n    import preopens;\n}\n" ;
const _ : & str = "\n/// This interface provides a value-export of the default network handle..\ninterface instance-network {\n    use network.{network};\n\n    /// Get a handle to the default network.\n    instance-network: func() -> network;\n\n}\n" ;
const _ : & str = "\ninterface ip-name-lookup {\n    use wasi:io/poll@0.2.0-rc-2023-11-10.{pollable};\n    use network.{network, error-code, ip-address};\n\n\n    /// Resolve an internet host name to a list of IP addresses.\n    ///\n    /// Unicode domain names are automatically converted to ASCII using IDNA encoding.\n    /// If the input is an IP address string, the address is parsed and returned\n    /// as-is without making any external requests.\n    ///\n    /// See the wasi-socket proposal README.md for a comparison with getaddrinfo.\n    ///\n    /// This function never blocks. It either immediately fails or immediately\n    /// returns successfully with a `resolve-address-stream` that can be used\n    /// to (asynchronously) fetch the results.\n    ///\n    /// # Typical errors\n    /// - `invalid-argument`: `name` is a syntactically invalid domain name or IP address.\n    ///\n    /// # References:\n    /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/getaddrinfo.html>\n    /// - <https://man7.org/linux/man-pages/man3/getaddrinfo.3.html>\n    /// - <https://learn.microsoft.com/en-us/windows/win32/api/ws2tcpip/nf-ws2tcpip-getaddrinfo>\n    /// - <https://man.freebsd.org/cgi/man.cgi?query=getaddrinfo&sektion=3>\n    resolve-addresses: func(network: borrow<network>, name: string) -> result<resolve-address-stream, error-code>;\n\n    resource resolve-address-stream {\n        /// Returns the next address from the resolver.\n        ///\n        /// This function should be called multiple times. On each call, it will\n        /// return the next address in connection order preference. If all\n        /// addresses have been exhausted, this function returns `none`.\n        ///\n        /// This function never returns IPv4-mapped IPv6 addresses.\n        ///\n        /// # Typical errors\n        /// - `name-unresolvable`:          Name does not exist or has no suitable associated IP addresses. (EAI_NONAME, EAI_NODATA, EAI_ADDRFAMILY)\n        /// - `temporary-resolver-failure`: A temporary failure in name resolution occurred. (EAI_AGAIN)\n        /// - `permanent-resolver-failure`: A permanent failure in name resolution occurred. (EAI_FAIL)\n        /// - `would-block`:                A result is not available yet. (EWOULDBLOCK, EAGAIN)\n        resolve-next-address: func() -> result<option<ip-address>, error-code>;\n\n        /// Create a `pollable` which will resolve once the stream is ready for I/O.\n        ///\n        /// Note: this function is here for WASI Preview2 only.\n        /// It\'s planned to be removed when `future` is natively supported in Preview3.\n        subscribe: func() -> pollable;\n    }\n}\n" ;
const _ : & str = "\ninterface network {\n    /// An opaque resource that represents access to (a subset of) the network.\n    /// This enables context-based security for networking.\n    /// There is no need for this to map 1:1 to a physical network interface.\n    resource network;\n\n    /// Error codes.\n    ///\n    /// In theory, every API can return any error code.\n    /// In practice, API\'s typically only return the errors documented per API\n    /// combined with a couple of errors that are always possible:\n    /// - `unknown`\n    /// - `access-denied`\n    /// - `not-supported`\n    /// - `out-of-memory`\n    /// - `concurrency-conflict`\n    ///\n    /// See each individual API for what the POSIX equivalents are. They sometimes differ per API.\n    enum error-code {\n        // ### GENERAL ERRORS ###\n\n        /// Unknown error\n        unknown,\n\n        /// Access denied.\n        ///\n        /// POSIX equivalent: EACCES, EPERM\n        access-denied,\n\n        /// The operation is not supported.\n        ///\n        /// POSIX equivalent: EOPNOTSUPP\n        not-supported,\n\n        /// One of the arguments is invalid.\n        ///\n        /// POSIX equivalent: EINVAL\n        invalid-argument,\n\n        /// Not enough memory to complete the operation.\n        ///\n        /// POSIX equivalent: ENOMEM, ENOBUFS, EAI_MEMORY\n        out-of-memory,\n\n        /// The operation timed out before it could finish completely.\n        timeout,\n\n        /// This operation is incompatible with another asynchronous operation that is already in progress.\n        ///\n        /// POSIX equivalent: EALREADY\n        concurrency-conflict,\n\n        /// Trying to finish an asynchronous operation that:\n        /// - has not been started yet, or:\n        /// - was already finished by a previous `finish-*` call.\n        ///\n        /// Note: this is scheduled to be removed when `future`s are natively supported.\n        not-in-progress,\n\n        /// The operation has been aborted because it could not be completed immediately.\n        ///\n        /// Note: this is scheduled to be removed when `future`s are natively supported.\n        would-block,\n\n\n\n        // ### TCP & UDP SOCKET ERRORS ###\n\n        /// The operation is not valid in the socket\'s current state.\n        invalid-state,\n\n        /// A new socket resource could not be created because of a system limit.\n        new-socket-limit,\n\n        /// A bind operation failed because the provided address is not an address that the `network` can bind to.\n        address-not-bindable,\n\n        /// A bind operation failed because the provided address is already in use or because there are no ephemeral ports available.\n        address-in-use,\n\n        /// The remote address is not reachable\n        remote-unreachable,\n\n\n        // ### TCP SOCKET ERRORS ###\n\n        /// The connection was forcefully rejected\n        connection-refused,\n\n        /// The connection was reset.\n        connection-reset,\n\n        /// A connection was aborted.\n        connection-aborted,\n\n\n        // ### UDP SOCKET ERRORS ###\n        datagram-too-large,\n\n\n        // ### NAME LOOKUP ERRORS ###\n\n        /// Name does not exist or has no suitable associated IP addresses.\n        name-unresolvable,\n\n        /// A temporary failure in name resolution occurred.\n        temporary-resolver-failure,\n\n        /// A permanent failure in name resolution occurred.\n        permanent-resolver-failure,\n    }\n\n    enum ip-address-family {\n        /// Similar to `AF_INET` in POSIX.\n        ipv4,\n\n        /// Similar to `AF_INET6` in POSIX.\n        ipv6,\n    }\n\n    type ipv4-address = tuple<u8, u8, u8, u8>;\n    type ipv6-address = tuple<u16, u16, u16, u16, u16, u16, u16, u16>;\n\n    variant ip-address {\n        ipv4(ipv4-address),\n        ipv6(ipv6-address),\n    }\n\n    record ipv4-socket-address {\n        port: u16, // sin_port\n        address: ipv4-address, // sin_addr\n    }\n\n    record ipv6-socket-address {\n        port: u16, // sin6_port\n        flow-info: u32, // sin6_flowinfo\n        address: ipv6-address, // sin6_addr\n        scope-id: u32, // sin6_scope_id\n    }\n\n    variant ip-socket-address {\n        ipv4(ipv4-socket-address),\n        ipv6(ipv6-socket-address),\n    }\n\n}\n" ;
const _ : & str = "\ninterface tcp-create-socket {\n    use network.{network, error-code, ip-address-family};\n    use tcp.{tcp-socket};\n\n    /// Create a new TCP socket.\n    ///\n    /// Similar to `socket(AF_INET or AF_INET6, SOCK_STREAM, IPPROTO_TCP)` in POSIX.\n    ///\n    /// This function does not require a network capability handle. This is considered to be safe because\n    /// at time of creation, the socket is not bound to any `network` yet. Up to the moment `bind`/`listen`/`connect`\n    /// is called, the socket is effectively an in-memory configuration object, unable to communicate with the outside world.\n    ///\n    /// All sockets are non-blocking. Use the wasi-poll interface to block on asynchronous operations.\n    ///\n    /// # Typical errors\n    /// - `not-supported`:     The specified `address-family` is not supported. (EAFNOSUPPORT)\n    /// - `new-socket-limit`:  The new socket resource could not be created because of a system limit. (EMFILE, ENFILE)\n    ///\n    /// # References\n    /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/socket.html>\n    /// - <https://man7.org/linux/man-pages/man2/socket.2.html>\n    /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-wsasocketw>\n    /// - <https://man.freebsd.org/cgi/man.cgi?query=socket&sektion=2>\n    create-tcp-socket: func(address-family: ip-address-family) -> result<tcp-socket, error-code>;\n}\n" ;
const _ : & str = "\ninterface tcp {\n    use wasi:io/streams@0.2.0-rc-2023-11-10.{input-stream, output-stream};\n    use wasi:io/poll@0.2.0-rc-2023-11-10.{pollable};\n    use wasi:clocks/monotonic-clock@0.2.0-rc-2023-11-10.{duration};\n    use network.{network, error-code, ip-socket-address, ip-address-family};\n\n    enum shutdown-type {\n        /// Similar to `SHUT_RD` in POSIX.\n        receive,\n\n        /// Similar to `SHUT_WR` in POSIX.\n        send,\n\n        /// Similar to `SHUT_RDWR` in POSIX.\n        both,\n    }\n\n\n    /// A TCP socket handle.\n    resource tcp-socket {\n        /// Bind the socket to a specific network on the provided IP address and port.\n        ///\n        /// If the IP address is zero (`0.0.0.0` in IPv4, `::` in IPv6), it is left to the implementation to decide which\n        /// network interface(s) to bind to.\n        /// If the TCP/UDP port is zero, the socket will be bound to a random free port.\n        ///\n        /// When a socket is not explicitly bound, the first invocation to a listen or connect operation will\n        /// implicitly bind the socket.\n        ///\n        /// Unlike in POSIX, this function is async. This enables interactive WASI hosts to inject permission prompts.\n        ///\n        /// # Typical `start` errors\n        /// - `invalid-argument`:          The `local-address` has the wrong address family. (EAFNOSUPPORT, EFAULT on Windows)\n        /// - `invalid-argument`:          `local-address` is not a unicast address. (EINVAL)\n        /// - `invalid-argument`:          `local-address` is an IPv4-mapped IPv6 address, but the socket has `ipv6-only` enabled. (EINVAL)\n        /// - `invalid-state`:             The socket is already bound. (EINVAL)\n        ///\n        /// # Typical `finish` errors\n        /// - `address-in-use`:            No ephemeral ports available. (EADDRINUSE, ENOBUFS on Windows)\n        /// - `address-in-use`:            Address is already in use. (EADDRINUSE)\n        /// - `address-not-bindable`:      `local-address` is not an address that the `network` can bind to. (EADDRNOTAVAIL)\n        /// - `not-in-progress`:           A `bind` operation is not in progress.\n        /// - `would-block`:               Can\'t finish the operation, it is still in progress. (EWOULDBLOCK, EAGAIN)\n        ///\n        /// # References\n        /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/bind.html>\n        /// - <https://man7.org/linux/man-pages/man2/bind.2.html>\n        /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock/nf-winsock-bind>\n        /// - <https://man.freebsd.org/cgi/man.cgi?query=bind&sektion=2&format=html>\n        start-bind: func(network: borrow<network>, local-address: ip-socket-address) -> result<_, error-code>;\n        finish-bind: func() -> result<_, error-code>;\n\n        /// Connect to a remote endpoint.\n        ///\n        /// On success:\n        /// - the socket is transitioned into the Connection state\n        /// - a pair of streams is returned that can be used to read & write to the connection\n        ///\n        /// POSIX mentions:\n        /// > If connect() fails, the state of the socket is unspecified. Conforming applications should\n        /// > close the file descriptor and create a new socket before attempting to reconnect.\n        ///\n        /// WASI prescribes the following behavior:\n        /// - If `connect` fails because an input/state validation error, the socket should remain usable.\n        /// - If a connection was actually attempted but failed, the socket should become unusable for further network communication.\n        ///   Besides `drop`, any method after such a failure may return an error.\n        ///\n        /// # Typical `start` errors\n        /// - `invalid-argument`:          The `remote-address` has the wrong address family. (EAFNOSUPPORT)\n        /// - `invalid-argument`:          `remote-address` is not a unicast address. (EINVAL, ENETUNREACH on Linux, EAFNOSUPPORT on MacOS)\n        /// - `invalid-argument`:          `remote-address` is an IPv4-mapped IPv6 address, but the socket has `ipv6-only` enabled. (EINVAL, EADDRNOTAVAIL on Illumos)\n        /// - `invalid-argument`:          `remote-address` is a non-IPv4-mapped IPv6 address, but the socket was bound to a specific IPv4-mapped IPv6 address. (or vice versa)\n        /// - `invalid-argument`:          The IP address in `remote-address` is set to INADDR_ANY (`0.0.0.0` / `::`). (EADDRNOTAVAIL on Windows)\n        /// - `invalid-argument`:          The port in `remote-address` is set to 0. (EADDRNOTAVAIL on Windows)\n        /// - `invalid-argument`:          The socket is already attached to a different network. The `network` passed to `connect` must be identical to the one passed to `bind`.\n        /// - `invalid-state`:             The socket is already in the Connection state. (EISCONN)\n        /// - `invalid-state`:             The socket is already in the Listener state. (EOPNOTSUPP, EINVAL on Windows)\n        ///\n        /// # Typical `finish` errors\n        /// - `timeout`:                   Connection timed out. (ETIMEDOUT)\n        /// - `connection-refused`:        The connection was forcefully rejected. (ECONNREFUSED)\n        /// - `connection-reset`:          The connection was reset. (ECONNRESET)\n        /// - `connection-aborted`:        The connection was aborted. (ECONNABORTED)\n        /// - `remote-unreachable`:        The remote address is not reachable. (EHOSTUNREACH, EHOSTDOWN, ENETUNREACH, ENETDOWN, ENONET)\n        /// - `address-in-use`:            Tried to perform an implicit bind, but there were no ephemeral ports available. (EADDRINUSE, EADDRNOTAVAIL on Linux, EAGAIN on BSD)\n        /// - `not-in-progress`:           A `connect` operation is not in progress.\n        /// - `would-block`:               Can\'t finish the operation, it is still in progress. (EWOULDBLOCK, EAGAIN)\n        ///\n        /// # References\n        /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/connect.html>\n        /// - <https://man7.org/linux/man-pages/man2/connect.2.html>\n        /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-connect>\n        /// - <https://man.freebsd.org/cgi/man.cgi?connect>\n        start-connect: func(network: borrow<network>, remote-address: ip-socket-address) -> result<_, error-code>;\n        finish-connect: func() -> result<tuple<input-stream, output-stream>, error-code>;\n\n        /// Start listening for new connections.\n        ///\n        /// Transitions the socket into the Listener state.\n        ///\n        /// Unlike POSIX:\n        /// - this function is async. This enables interactive WASI hosts to inject permission prompts.\n        /// - the socket must already be explicitly bound.\n        ///\n        /// # Typical `start` errors\n        /// - `invalid-state`:             The socket is not bound to any local address. (EDESTADDRREQ)\n        /// - `invalid-state`:             The socket is already in the Connection state. (EISCONN, EINVAL on BSD)\n        /// - `invalid-state`:             The socket is already in the Listener state.\n        ///\n        /// # Typical `finish` errors\n        /// - `address-in-use`:            Tried to perform an implicit bind, but there were no ephemeral ports available. (EADDRINUSE)\n        /// - `not-in-progress`:           A `listen` operation is not in progress.\n        /// - `would-block`:               Can\'t finish the operation, it is still in progress. (EWOULDBLOCK, EAGAIN)\n        ///\n        /// # References\n        /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/listen.html>\n        /// - <https://man7.org/linux/man-pages/man2/listen.2.html>\n        /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-listen>\n        /// - <https://man.freebsd.org/cgi/man.cgi?query=listen&sektion=2>\n        start-listen: func() -> result<_, error-code>;\n        finish-listen: func() -> result<_, error-code>;\n\n        /// Accept a new client socket.\n        ///\n        /// The returned socket is bound and in the Connection state. The following properties are inherited from the listener socket:\n        /// - `address-family`\n        /// - `ipv6-only`\n        /// - `keep-alive-enabled`\n        /// - `keep-alive-idle-time`\n        /// - `keep-alive-interval`\n        /// - `keep-alive-count`\n        /// - `hop-limit`\n        /// - `receive-buffer-size`\n        /// - `send-buffer-size`\n        ///\n        /// On success, this function returns the newly accepted client socket along with\n        /// a pair of streams that can be used to read & write to the connection.\n        ///\n        /// # Typical errors\n        /// - `invalid-state`:      Socket is not in the Listener state. (EINVAL)\n        /// - `would-block`:        No pending connections at the moment. (EWOULDBLOCK, EAGAIN)\n        /// - `connection-aborted`: An incoming connection was pending, but was terminated by the client before this listener could accept it. (ECONNABORTED)\n        /// - `new-socket-limit`:   The new socket resource could not be created because of a system limit. (EMFILE, ENFILE)\n        ///\n        /// # References\n        /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/accept.html>\n        /// - <https://man7.org/linux/man-pages/man2/accept.2.html>\n        /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-accept>\n        /// - <https://man.freebsd.org/cgi/man.cgi?query=accept&sektion=2>\n        accept: func() -> result<tuple<tcp-socket, input-stream, output-stream>, error-code>;\n\n        /// Get the bound local address.\n        ///\n        /// POSIX mentions:\n        /// > If the socket has not been bound to a local name, the value\n        /// > stored in the object pointed to by `address` is unspecified.\n        ///\n        /// WASI is stricter and requires `local-address` to return `invalid-state` when the socket hasn\'t been bound yet.\n        ///\n        /// # Typical errors\n        /// - `invalid-state`: The socket is not bound to any local address.\n        ///\n        /// # References\n        /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/getsockname.html>\n        /// - <https://man7.org/linux/man-pages/man2/getsockname.2.html>\n        /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock/nf-winsock-getsockname>\n        /// - <https://man.freebsd.org/cgi/man.cgi?getsockname>\n        local-address: func() -> result<ip-socket-address, error-code>;\n\n        /// Get the remote address.\n        ///\n        /// # Typical errors\n        /// - `invalid-state`: The socket is not connected to a remote address. (ENOTCONN)\n        ///\n        /// # References\n        /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/getpeername.html>\n        /// - <https://man7.org/linux/man-pages/man2/getpeername.2.html>\n        /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock/nf-winsock-getpeername>\n        /// - <https://man.freebsd.org/cgi/man.cgi?query=getpeername&sektion=2&n=1>\n        remote-address: func() -> result<ip-socket-address, error-code>;\n\n        /// Whether the socket is listening for new connections.\n        ///\n        /// Equivalent to the SO_ACCEPTCONN socket option.\n        is-listening: func() -> bool;\n\n        /// Whether this is a IPv4 or IPv6 socket.\n        ///\n        /// Equivalent to the SO_DOMAIN socket option.\n        address-family: func() -> ip-address-family;\n\n        /// Whether IPv4 compatibility (dual-stack) mode is disabled or not.\n        ///\n        /// Equivalent to the IPV6_V6ONLY socket option.\n        ///\n        /// # Typical errors\n        /// - `invalid-state`:        (set) The socket is already bound.\n        /// - `not-supported`:        (get/set) `this` socket is an IPv4 socket.\n        /// - `not-supported`:        (set) Host does not support dual-stack sockets. (Implementations are not required to.)\n        ipv6-only: func() -> result<bool, error-code>;\n        set-ipv6-only: func(value: bool) -> result<_, error-code>;\n\n        /// Hints the desired listen queue size. Implementations are free to ignore this.\n        ///\n        /// If the provided value is 0, an `invalid-argument` error is returned.\n        /// Any other value will never cause an error, but it might be silently clamped and/or rounded.\n        ///\n        /// # Typical errors\n        /// - `not-supported`:        (set) The platform does not support changing the backlog size after the initial listen.\n        /// - `invalid-argument`:     (set) The provided value was 0.\n        /// - `invalid-state`:        (set) The socket is already in the Connection state.\n        set-listen-backlog-size: func(value: u64) -> result<_, error-code>;\n\n        /// Enables or disables keepalive.\n        ///\n        /// The keepalive behavior can be adjusted using:\n        /// - `keep-alive-idle-time`\n        /// - `keep-alive-interval`\n        /// - `keep-alive-count`\n        /// These properties can be configured while `keep-alive-enabled` is false, but only come into effect when `keep-alive-enabled` is true.\n        ///\n        /// Equivalent to the SO_KEEPALIVE socket option.\n        keep-alive-enabled: func() -> result<bool, error-code>;\n        set-keep-alive-enabled: func(value: bool) -> result<_, error-code>;\n\n        /// Amount of time the connection has to be idle before TCP starts sending keepalive packets.\n        ///\n        /// If the provided value is 0, an `invalid-argument` error is returned.\n        /// Any other value will never cause an error, but it might be silently clamped and/or rounded.\n        /// I.e. after setting a value, reading the same setting back may return a different value.\n        ///\n        /// Equivalent to the TCP_KEEPIDLE socket option. (TCP_KEEPALIVE on MacOS)\n        ///\n        /// # Typical errors\n        /// - `invalid-argument`:     (set) The provided value was 0.\n        keep-alive-idle-time: func() -> result<duration, error-code>;\n        set-keep-alive-idle-time: func(value: duration) -> result<_, error-code>;\n\n        /// The time between keepalive packets.\n        ///\n        /// If the provided value is 0, an `invalid-argument` error is returned.\n        /// Any other value will never cause an error, but it might be silently clamped and/or rounded.\n        /// I.e. after setting a value, reading the same setting back may return a different value.\n        ///\n        /// Equivalent to the TCP_KEEPINTVL socket option.\n        ///\n        /// # Typical errors\n        /// - `invalid-argument`:     (set) The provided value was 0.\n        keep-alive-interval: func() -> result<duration, error-code>;\n        set-keep-alive-interval: func(value: duration) -> result<_, error-code>;\n\n        /// The maximum amount of keepalive packets TCP should send before aborting the connection.\n        ///\n        /// If the provided value is 0, an `invalid-argument` error is returned.\n        /// Any other value will never cause an error, but it might be silently clamped and/or rounded.\n        /// I.e. after setting a value, reading the same setting back may return a different value.\n        ///\n        /// Equivalent to the TCP_KEEPCNT socket option.\n        ///\n        /// # Typical errors\n        /// - `invalid-argument`:     (set) The provided value was 0.\n        keep-alive-count: func() -> result<u32, error-code>;\n        set-keep-alive-count: func(value: u32) -> result<_, error-code>;\n\n        /// Equivalent to the IP_TTL & IPV6_UNICAST_HOPS socket options.\n        ///\n        /// If the provided value is 0, an `invalid-argument` error is returned.\n        ///\n        /// # Typical errors\n        /// - `invalid-argument`:     (set) The TTL value must be 1 or higher.\n        /// - `invalid-state`:        (set) The socket is already in the Connection state.\n        /// - `invalid-state`:        (set) The socket is already in the Listener state.\n        hop-limit: func() -> result<u8, error-code>;\n        set-hop-limit: func(value: u8) -> result<_, error-code>;\n\n        /// The kernel buffer space reserved for sends/receives on this socket.\n        ///\n        /// If the provided value is 0, an `invalid-argument` error is returned.\n        /// Any other value will never cause an error, but it might be silently clamped and/or rounded.\n        /// I.e. after setting a value, reading the same setting back may return a different value.\n        ///\n        /// Equivalent to the SO_RCVBUF and SO_SNDBUF socket options.\n        ///\n        /// # Typical errors\n        /// - `invalid-argument`:     (set) The provided value was 0.\n        /// - `invalid-state`:        (set) The socket is already in the Connection state.\n        /// - `invalid-state`:        (set) The socket is already in the Listener state.\n        receive-buffer-size: func() -> result<u64, error-code>;\n        set-receive-buffer-size: func(value: u64) -> result<_, error-code>;\n        send-buffer-size: func() -> result<u64, error-code>;\n        set-send-buffer-size: func(value: u64) -> result<_, error-code>;\n\n        /// Create a `pollable` which will resolve once the socket is ready for I/O.\n        ///\n        /// Note: this function is here for WASI Preview2 only.\n        /// It\'s planned to be removed when `future` is natively supported in Preview3.\n        subscribe: func() -> pollable;\n\n        /// Initiate a graceful shutdown.\n        ///\n        /// - receive: the socket is not expecting to receive any more data from the peer. All subsequent read\n        ///   operations on the `input-stream` associated with this socket will return an End Of Stream indication.\n        ///   Any data still in the receive queue at time of calling `shutdown` will be discarded.\n        /// - send: the socket is not expecting to send any more data to the peer. All subsequent write\n        ///   operations on the `output-stream` associated with this socket will return an error.\n        /// - both: same effect as receive & send combined.\n        ///\n        /// The shutdown function does not close (drop) the socket.\n        ///\n        /// # Typical errors\n        /// - `invalid-state`: The socket is not in the Connection state. (ENOTCONN)\n        ///\n        /// # References\n        /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/shutdown.html>\n        /// - <https://man7.org/linux/man-pages/man2/shutdown.2.html>\n        /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock/nf-winsock-shutdown>\n        /// - <https://man.freebsd.org/cgi/man.cgi?query=shutdown&sektion=2>\n        shutdown: func(shutdown-type: shutdown-type) -> result<_, error-code>;\n    }\n}\n" ;
const _ : & str = "\ninterface udp-create-socket {\n    use network.{network, error-code, ip-address-family};\n    use udp.{udp-socket};\n\n    /// Create a new UDP socket.\n    ///\n    /// Similar to `socket(AF_INET or AF_INET6, SOCK_DGRAM, IPPROTO_UDP)` in POSIX.\n    ///\n    /// This function does not require a network capability handle. This is considered to be safe because\n    /// at time of creation, the socket is not bound to any `network` yet. Up to the moment `bind` is called,\n    /// the socket is effectively an in-memory configuration object, unable to communicate with the outside world.\n    ///\n    /// All sockets are non-blocking. Use the wasi-poll interface to block on asynchronous operations.\n    ///\n    /// # Typical errors\n    /// - `not-supported`:     The specified `address-family` is not supported. (EAFNOSUPPORT)\n    /// - `new-socket-limit`:  The new socket resource could not be created because of a system limit. (EMFILE, ENFILE)\n    ///\n    /// # References:\n    /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/socket.html>\n    /// - <https://man7.org/linux/man-pages/man2/socket.2.html>\n    /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-wsasocketw>\n    /// - <https://man.freebsd.org/cgi/man.cgi?query=socket&sektion=2>\n    create-udp-socket: func(address-family: ip-address-family) -> result<udp-socket, error-code>;\n}\n" ;
const _ : & str = "\ninterface udp {\n    use wasi:io/poll@0.2.0-rc-2023-11-10.{pollable};\n    use network.{network, error-code, ip-socket-address, ip-address-family};\n\n    /// A received datagram.\n    record incoming-datagram {\n        /// The payload.\n        /// \n        /// Theoretical max size: ~64 KiB. In practice, typically less than 1500 bytes.\n        data: list<u8>,\n\n        /// The source address.\n        ///\n        /// This field is guaranteed to match the remote address the stream was initialized with, if any.\n        ///\n        /// Equivalent to the `src_addr` out parameter of `recvfrom`.\n        remote-address: ip-socket-address,\n    }\n\n    /// A datagram to be sent out.\n    record outgoing-datagram {\n        /// The payload.\n        data: list<u8>,\n\n        /// The destination address.\n        ///\n        /// The requirements on this field depend on how the stream was initialized:\n        /// - with a remote address: this field must be None or match the stream\'s remote address exactly.\n        /// - without a remote address: this field is required.\n        ///\n        /// If this value is None, the send operation is equivalent to `send` in POSIX. Otherwise it is equivalent to `sendto`.\n        remote-address: option<ip-socket-address>,\n    }\n\n\n\n    /// A UDP socket handle.\n    resource udp-socket {\n        /// Bind the socket to a specific network on the provided IP address and port.\n        ///\n        /// If the IP address is zero (`0.0.0.0` in IPv4, `::` in IPv6), it is left to the implementation to decide which\n        /// network interface(s) to bind to.\n        /// If the port is zero, the socket will be bound to a random free port.\n        ///\n        /// Unlike in POSIX, this function is async. This enables interactive WASI hosts to inject permission prompts.\n        ///\n        /// # Typical `start` errors\n        /// - `invalid-argument`:          The `local-address` has the wrong address family. (EAFNOSUPPORT, EFAULT on Windows)\n        /// - `invalid-state`:             The socket is already bound. (EINVAL)\n        ///\n        /// # Typical `finish` errors\n        /// - `address-in-use`:            No ephemeral ports available. (EADDRINUSE, ENOBUFS on Windows)\n        /// - `address-in-use`:            Address is already in use. (EADDRINUSE)\n        /// - `address-not-bindable`:      `local-address` is not an address that the `network` can bind to. (EADDRNOTAVAIL)\n        /// - `not-in-progress`:           A `bind` operation is not in progress.\n        /// - `would-block`:               Can\'t finish the operation, it is still in progress. (EWOULDBLOCK, EAGAIN)\n        ///\n        /// # References\n        /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/bind.html>\n        /// - <https://man7.org/linux/man-pages/man2/bind.2.html>\n        /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock/nf-winsock-bind>\n        /// - <https://man.freebsd.org/cgi/man.cgi?query=bind&sektion=2&format=html>\n        start-bind: func(network: borrow<network>, local-address: ip-socket-address) -> result<_, error-code>;\n        finish-bind: func() -> result<_, error-code>;\n\n        /// Set up inbound & outbound communication channels, optionally to a specific peer.\n        ///\n        /// This function only changes the local socket configuration and does not generate any network traffic.\n        /// On success, the `remote-address` of the socket is updated. The `local-address` may be updated as well,\n        /// based on the best network path to `remote-address`.\n        ///\n        /// When a `remote-address` is provided, the returned streams are limited to communicating with that specific peer:\n        /// - `send` can only be used to send to this destination.\n        /// - `receive` will only return datagrams sent from the provided `remote-address`.\n        ///\n        /// This method may be called multiple times on the same socket to change its association, but\n        /// only the most recently returned pair of streams will be operational. Implementations may trap if\n        /// the streams returned by a previous invocation haven\'t been dropped yet before calling `stream` again.\n        /// \n        /// The POSIX equivalent in pseudo-code is:\n        /// ```text\n        /// if (was previously connected) {\n        /// \tconnect(s, AF_UNSPEC)\n        /// }\n        /// if (remote_address is Some) {\n        /// \tconnect(s, remote_address)\n        /// }\n        /// ```\n        ///\n        /// Unlike in POSIX, the socket must already be explicitly bound.\n        /// \n        /// # Typical errors\n        /// - `invalid-argument`:          The `remote-address` has the wrong address family. (EAFNOSUPPORT)\n        /// - `invalid-argument`:          `remote-address` is a non-IPv4-mapped IPv6 address, but the socket was bound to a specific IPv4-mapped IPv6 address. (or vice versa)\n        /// - `invalid-argument`:          The IP address in `remote-address` is set to INADDR_ANY (`0.0.0.0` / `::`). (EDESTADDRREQ, EADDRNOTAVAIL)\n        /// - `invalid-argument`:          The port in `remote-address` is set to 0. (EDESTADDRREQ, EADDRNOTAVAIL)\n        /// - `invalid-state`:             The socket is not bound.\n        /// - `address-in-use`:            Tried to perform an implicit bind, but there were no ephemeral ports available. (EADDRINUSE, EADDRNOTAVAIL on Linux, EAGAIN on BSD)\n        /// - `remote-unreachable`:        The remote address is not reachable. (ECONNRESET, ENETRESET, EHOSTUNREACH, EHOSTDOWN, ENETUNREACH, ENETDOWN, ENONET)\n        /// - `connection-refused`:        The connection was refused. (ECONNREFUSED)\n        ///\n        /// # References\n        /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/connect.html>\n        /// - <https://man7.org/linux/man-pages/man2/connect.2.html>\n        /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-connect>\n        /// - <https://man.freebsd.org/cgi/man.cgi?connect>\n        %stream: func(remote-address: option<ip-socket-address>) -> result<tuple<incoming-datagram-stream, outgoing-datagram-stream>, error-code>;\n\n        /// Get the current bound address.\n        ///\n        /// POSIX mentions:\n        /// > If the socket has not been bound to a local name, the value\n        /// > stored in the object pointed to by `address` is unspecified.\n        ///\n        /// WASI is stricter and requires `local-address` to return `invalid-state` when the socket hasn\'t been bound yet.\n        /// \n        /// # Typical errors\n        /// - `invalid-state`: The socket is not bound to any local address.\n        ///\n        /// # References\n        /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/getsockname.html>\n        /// - <https://man7.org/linux/man-pages/man2/getsockname.2.html>\n        /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock/nf-winsock-getsockname>\n        /// - <https://man.freebsd.org/cgi/man.cgi?getsockname>\n        local-address: func() -> result<ip-socket-address, error-code>;\n\n        /// Get the address the socket is currently streaming to.\n        ///\n        /// # Typical errors\n        /// - `invalid-state`: The socket is not streaming to a specific remote address. (ENOTCONN)\n        ///\n        /// # References\n        /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/getpeername.html>\n        /// - <https://man7.org/linux/man-pages/man2/getpeername.2.html>\n        /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock/nf-winsock-getpeername>\n        /// - <https://man.freebsd.org/cgi/man.cgi?query=getpeername&sektion=2&n=1>\n        remote-address: func() -> result<ip-socket-address, error-code>;\n\n        /// Whether this is a IPv4 or IPv6 socket.\n        ///\n        /// Equivalent to the SO_DOMAIN socket option.\n        address-family: func() -> ip-address-family;\n\n        /// Whether IPv4 compatibility (dual-stack) mode is disabled or not.\n        ///\n        /// Equivalent to the IPV6_V6ONLY socket option.\n        ///\n        /// # Typical errors\n        /// - `not-supported`:        (get/set) `this` socket is an IPv4 socket.\n        /// - `invalid-state`:        (set) The socket is already bound.\n        /// - `not-supported`:        (set) Host does not support dual-stack sockets. (Implementations are not required to.)\n        ipv6-only: func() -> result<bool, error-code>;\n        set-ipv6-only: func(value: bool) -> result<_, error-code>;\n\n        /// Equivalent to the IP_TTL & IPV6_UNICAST_HOPS socket options.\n        ///\n        /// If the provided value is 0, an `invalid-argument` error is returned.\n        ///\n        /// # Typical errors\n        /// - `invalid-argument`:     (set) The TTL value must be 1 or higher.\n        unicast-hop-limit: func() -> result<u8, error-code>;\n        set-unicast-hop-limit: func(value: u8) -> result<_, error-code>;\n\n        /// The kernel buffer space reserved for sends/receives on this socket.\n        ///\n        /// If the provided value is 0, an `invalid-argument` error is returned.\n        /// Any other value will never cause an error, but it might be silently clamped and/or rounded.\n        /// I.e. after setting a value, reading the same setting back may return a different value.\n        ///\n        /// Equivalent to the SO_RCVBUF and SO_SNDBUF socket options.\n        ///\n        /// # Typical errors\n        /// - `invalid-argument`:     (set) The provided value was 0.\n        receive-buffer-size: func() -> result<u64, error-code>;\n        set-receive-buffer-size: func(value: u64) -> result<_, error-code>;\n        send-buffer-size: func() -> result<u64, error-code>;\n        set-send-buffer-size: func(value: u64) -> result<_, error-code>;\n\n        /// Create a `pollable` which will resolve once the socket is ready for I/O.\n        ///\n        /// Note: this function is here for WASI Preview2 only.\n        /// It\'s planned to be removed when `future` is natively supported in Preview3.\n        subscribe: func() -> pollable;\n    }\n\n    resource incoming-datagram-stream {\n        /// Receive messages on the socket.\n        ///\n        /// This function attempts to receive up to `max-results` datagrams on the socket without blocking.\n        /// The returned list may contain fewer elements than requested, but never more.\n        ///\n        /// This function returns successfully with an empty list when either:\n        /// - `max-results` is 0, or:\n        /// - `max-results` is greater than 0, but no results are immediately available.\n        /// This function never returns `error(would-block)`.\n        ///\n        /// # Typical errors\n        /// - `remote-unreachable`: The remote address is not reachable. (ECONNRESET, ENETRESET on Windows, EHOSTUNREACH, EHOSTDOWN, ENETUNREACH, ENETDOWN, ENONET)\n        /// - `connection-refused`: The connection was refused. (ECONNREFUSED)\n        ///\n        /// # References\n        /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/recvfrom.html>\n        /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/recvmsg.html>\n        /// - <https://man7.org/linux/man-pages/man2/recv.2.html>\n        /// - <https://man7.org/linux/man-pages/man2/recvmmsg.2.html>\n        /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock/nf-winsock-recv>\n        /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock/nf-winsock-recvfrom>\n        /// - <https://learn.microsoft.com/en-us/previous-versions/windows/desktop/legacy/ms741687(v=vs.85)>\n        /// - <https://man.freebsd.org/cgi/man.cgi?query=recv&sektion=2>\n        receive: func(max-results: u64) -> result<list<incoming-datagram>, error-code>;\n\n        /// Create a `pollable` which will resolve once the stream is ready to receive again.\n        ///\n        /// Note: this function is here for WASI Preview2 only.\n        /// It\'s planned to be removed when `future` is natively supported in Preview3.\n        subscribe: func() -> pollable;\n    }\n\n    resource outgoing-datagram-stream {\n        /// Check readiness for sending. This function never blocks.\n        ///\n        /// Returns the number of datagrams permitted for the next call to `send`,\n        /// or an error. Calling `send` with more datagrams than this function has\n        /// permitted will trap.\n        ///\n        /// When this function returns ok(0), the `subscribe` pollable will\n        /// become ready when this function will report at least ok(1), or an\n        /// error.\n        /// \n        /// Never returns `would-block`.\n        check-send: func() -> result<u64, error-code>;\n\n        /// Send messages on the socket.\n        ///\n        /// This function attempts to send all provided `datagrams` on the socket without blocking and\n        /// returns how many messages were actually sent (or queued for sending). This function never\n        /// returns `error(would-block)`. If none of the datagrams were able to be sent, `ok(0)` is returned.\n        ///\n        /// This function semantically behaves the same as iterating the `datagrams` list and sequentially\n        /// sending each individual datagram until either the end of the list has been reached or the first error occurred.\n        /// If at least one datagram has been sent successfully, this function never returns an error.\n        ///\n        /// If the input list is empty, the function returns `ok(0)`.\n        ///\n        /// Each call to `send` must be permitted by a preceding `check-send`. Implementations must trap if\n        /// either `check-send` was not called or `datagrams` contains more items than `check-send` permitted.\n        ///\n        /// # Typical errors\n        /// - `invalid-argument`:        The `remote-address` has the wrong address family. (EAFNOSUPPORT)\n        /// - `invalid-argument`:        `remote-address` is a non-IPv4-mapped IPv6 address, but the socket was bound to a specific IPv4-mapped IPv6 address. (or vice versa)\n        /// - `invalid-argument`:        The IP address in `remote-address` is set to INADDR_ANY (`0.0.0.0` / `::`). (EDESTADDRREQ, EADDRNOTAVAIL)\n        /// - `invalid-argument`:        The port in `remote-address` is set to 0. (EDESTADDRREQ, EADDRNOTAVAIL)\n        /// - `invalid-argument`:        The socket is in \"connected\" mode and `remote-address` is `some` value that does not match the address passed to `stream`. (EISCONN)\n        /// - `invalid-argument`:        The socket is not \"connected\" and no value for `remote-address` was provided. (EDESTADDRREQ)\n        /// - `remote-unreachable`:      The remote address is not reachable. (ECONNRESET, ENETRESET on Windows, EHOSTUNREACH, EHOSTDOWN, ENETUNREACH, ENETDOWN, ENONET)\n        /// - `connection-refused`:      The connection was refused. (ECONNREFUSED)\n        /// - `datagram-too-large`:      The datagram is too large. (EMSGSIZE)\n        ///\n        /// # References\n        /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/sendto.html>\n        /// - <https://pubs.opengroup.org/onlinepubs/9699919799/functions/sendmsg.html>\n        /// - <https://man7.org/linux/man-pages/man2/send.2.html>\n        /// - <https://man7.org/linux/man-pages/man2/sendmmsg.2.html>\n        /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-send>\n        /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-sendto>\n        /// - <https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-wsasendmsg>\n        /// - <https://man.freebsd.org/cgi/man.cgi?query=send&sektion=2>\n        send: func(datagrams: list<outgoing-datagram>) -> result<u64, error-code>;\n        \n        /// Create a `pollable` which will resolve once the stream is ready to send again.\n        ///\n        /// Note: this function is here for WASI Preview2 only.\n        /// It\'s planned to be removed when `future` is natively supported in Preview3.\n        subscribe: func() -> pollable;\n    }\n}\n" ;
const _ : & str = "package wasi:sockets@0.2.0-rc-2023-11-10;\n\nworld imports {\n    import instance-network;\n    import network;\n    import udp;\n    import udp-create-socket;\n    import tcp;\n    import tcp-create-socket;\n    import ip-name-lookup;\n}\n" ;
const _ : & str = "package wasi:random@0.2.0-rc-2023-11-10;\n/// The insecure-seed interface for seeding hash-map DoS resistance.\n///\n/// It is intended to be portable at least between Unix-family platforms and\n/// Windows.\ninterface insecure-seed {\n    /// Return a 128-bit value that may contain a pseudo-random value.\n    ///\n    /// The returned value is not required to be computed from a CSPRNG, and may\n    /// even be entirely deterministic. Host implementations are encouraged to\n    /// provide pseudo-random values to any program exposed to\n    /// attacker-controlled content, to enable DoS protection built into many\n    /// languages\' hash-map implementations.\n    ///\n    /// This function is intended to only be called once, by a source language\n    /// to initialize Denial Of Service (DoS) protection in its hash-map\n    /// implementation.\n    ///\n    /// # Expected future evolution\n    ///\n    /// This will likely be changed to a value import, to prevent it from being\n    /// called multiple times and potentially used for purposes other than DoS\n    /// protection.\n    insecure-seed: func() -> tuple<u64, u64>;\n}\n" ;
const _ : & str = "package wasi:random@0.2.0-rc-2023-11-10;\n/// The insecure interface for insecure pseudo-random numbers.\n///\n/// It is intended to be portable at least between Unix-family platforms and\n/// Windows.\ninterface insecure {\n    /// Return `len` insecure pseudo-random bytes.\n    ///\n    /// This function is not cryptographically secure. Do not use it for\n    /// anything related to security.\n    ///\n    /// There are no requirements on the values of the returned bytes, however\n    /// implementations are encouraged to return evenly distributed values with\n    /// a long period.\n    get-insecure-random-bytes: func(len: u64) -> list<u8>;\n\n    /// Return an insecure pseudo-random `u64` value.\n    ///\n    /// This function returns the same type of pseudo-random data as\n    /// `get-insecure-random-bytes`, represented as a `u64`.\n    get-insecure-random-u64: func() -> u64;\n}\n" ;
const _ : & str = "package wasi:random@0.2.0-rc-2023-11-10;\n/// WASI Random is a random data API.\n///\n/// It is intended to be portable at least between Unix-family platforms and\n/// Windows.\ninterface random {\n    /// Return `len` cryptographically-secure random or pseudo-random bytes.\n    ///\n    /// This function must produce data at least as cryptographically secure and\n    /// fast as an adequately seeded cryptographically-secure pseudo-random\n    /// number generator (CSPRNG). It must not block, from the perspective of\n    /// the calling program, under any circumstances, including on the first\n    /// request and on requests for numbers of bytes. The returned data must\n    /// always be unpredictable.\n    ///\n    /// This function must always return fresh data. Deterministic environments\n    /// must omit this function, rather than implementing it with deterministic\n    /// data.\n    get-random-bytes: func(len: u64) -> list<u8>;\n\n    /// Return a cryptographically-secure random or pseudo-random `u64` value.\n    ///\n    /// This function returns the same type of data as `get-random-bytes`,\n    /// represented as a `u64`.\n    get-random-u64: func() -> u64;\n}\n" ;
const _ : & str = "package wasi:random@0.2.0-rc-2023-11-10;\n\nworld imports {\n    import random;\n    import insecure;\n    import insecure-seed;\n}\n" ;
const _ : & str = "package wasi:cli@0.2.0-rc-2023-12-05;\n\nworld command {\n  include imports;\n\n  export run;\n}\n" ;
const _ : & str = "interface environment {\n  /// Get the POSIX-style environment variables.\n  ///\n  /// Each environment variable is provided as a pair of string variable names\n  /// and string value.\n  ///\n  /// Morally, these are a value import, but until value imports are available\n  /// in the component model, this import function should return the same\n  /// values each time it is called.\n  get-environment: func() -> list<tuple<string, string>>;\n\n  /// Get the POSIX-style arguments to the program.\n  get-arguments: func() -> list<string>;\n\n  /// Return a path that programs should use as their initial current working\n  /// directory, interpreting `.` as shorthand for this.\n  initial-cwd: func() -> option<string>;\n}\n" ;
const _ : & str = "interface exit {\n  /// Exit the current instance and any linked instances.\n  exit: func(status: result);\n}\n" ;
const _ : & str = "package wasi:cli@0.2.0-rc-2023-12-05;\n\nworld imports {\n  include wasi:clocks/imports@0.2.0-rc-2023-11-10;\n  include wasi:filesystem/imports@0.2.0-rc-2023-11-10;\n  include wasi:sockets/imports@0.2.0-rc-2023-11-10;\n  include wasi:random/imports@0.2.0-rc-2023-11-10;\n  include wasi:io/imports@0.2.0-rc-2023-11-10;\n\n  import environment;\n  import exit;\n  import stdin;\n  import stdout;\n  import stderr;\n  import terminal-input;\n  import terminal-output;\n  import terminal-stdin;\n  import terminal-stdout;\n  import terminal-stderr;\n}\n" ;
const _: &str = "interface run {\n  /// Run the program.\n  run: func() -> result;\n}\n";
const _ : & str = "interface stdin {\n  use wasi:io/streams@0.2.0-rc-2023-11-10.{input-stream};\n\n  get-stdin: func() -> input-stream;\n}\n\ninterface stdout {\n  use wasi:io/streams@0.2.0-rc-2023-11-10.{output-stream};\n\n  get-stdout: func() -> output-stream;\n}\n\ninterface stderr {\n  use wasi:io/streams@0.2.0-rc-2023-11-10.{output-stream};\n\n  get-stderr: func() -> output-stream;\n}\n" ;
const _ : & str = "interface terminal-input {\n    /// The input side of a terminal.\n    resource terminal-input;\n\n    // In the future, this may include functions for disabling echoing,\n    // disabling input buffering so that keyboard events are sent through\n    // immediately, querying supported features, and so on.\n}\n\ninterface terminal-output {\n    /// The output side of a terminal.\n    resource terminal-output;\n\n    // In the future, this may include functions for querying the terminal\n    // size, being notified of terminal size changes, querying supported\n    // features, and so on.\n}\n\n/// An interface providing an optional `terminal-input` for stdin as a\n/// link-time authority.\ninterface terminal-stdin {\n    use terminal-input.{terminal-input};\n\n    /// If stdin is connected to a terminal, return a `terminal-input` handle\n    /// allowing further interaction with it.\n    get-terminal-stdin: func() -> option<terminal-input>;\n}\n\n/// An interface providing an optional `terminal-output` for stdout as a\n/// link-time authority.\ninterface terminal-stdout {\n    use terminal-output.{terminal-output};\n\n    /// If stdout is connected to a terminal, return a `terminal-output` handle\n    /// allowing further interaction with it.\n    get-terminal-stdout: func() -> option<terminal-output>;\n}\n\n/// An interface providing an optional `terminal-output` for stderr as a\n/// link-time authority.\ninterface terminal-stderr {\n    use terminal-output.{terminal-output};\n\n    /// If stderr is connected to a terminal, return a `terminal-output` handle\n    /// allowing further interaction with it.\n    get-terminal-stderr: func() -> option<terminal-output>;\n}\n" ;
const _ : & str = "/// This interface defines a handler of incoming HTTP Requests. It should\n/// be exported by components which can respond to HTTP Requests.\ninterface incoming-handler {\n  use types.{incoming-request, response-outparam};\n\n  /// This function is invoked with an incoming HTTP Request, and a resource\n  /// `response-outparam` which provides the capability to reply with an HTTP\n  /// Response. The response is sent by calling the `response-outparam.set`\n  /// method, which allows execution to continue after the response has been\n  /// sent. This enables both streaming to the response body, and performing other\n  /// work.\n  ///\n  /// The implementor of this function must write a response to the\n  /// `response-outparam` before returning, or else the caller will respond\n  /// with an error on its behalf.\n  handle: func(\n    request: incoming-request,\n    response-out: response-outparam\n  );\n}\n\n/// This interface defines a handler of outgoing HTTP Requests. It should be\n/// imported by components which wish to make HTTP Requests.\ninterface outgoing-handler {\n  use types.{\n    outgoing-request, request-options, future-incoming-response, error-code\n  };\n\n  /// This function is invoked with an outgoing HTTP Request, and it returns\n  /// a resource `future-incoming-response` which represents an HTTP Response\n  /// which may arrive in the future.\n  ///\n  /// The `options` argument accepts optional parameters for the HTTP\n  /// protocol\'s transport layer.\n  ///\n  /// This function may return an error if the `outgoing-request` is invalid\n  /// or not allowed to be made. Otherwise, protocol errors are reported\n  /// through the `future-incoming-response`.\n  handle: func(\n    request: outgoing-request,\n    options: option<request-options>\n  ) -> result<future-incoming-response, error-code>;\n}\n" ;
const _ : & str = "package wasi:http@0.2.0-rc-2023-12-05;\n\n/// The `wasi:http/proxy` world captures a widely-implementable intersection of\n/// hosts that includes HTTP forward and reverse proxies. Components targeting\n/// this world may concurrently stream in and out any number of incoming and\n/// outgoing HTTP requests.\nworld proxy {\n  /// HTTP proxies have access to time and randomness.\n  include wasi:clocks/imports@0.2.0-rc-2023-11-10;\n  import wasi:random/random@0.2.0-rc-2023-11-10;\n\n  /// Proxies have standard output and error streams which are expected to\n  /// terminate in a developer-facing console provided by the host.\n  import wasi:cli/stdout@0.2.0-rc-2023-12-05;\n  import wasi:cli/stderr@0.2.0-rc-2023-12-05;\n\n  /// TODO: this is a temporary workaround until component tooling is able to\n  /// gracefully handle the absence of stdin. Hosts must return an eof stream\n  /// for this import, which is what wasi-libc + tooling will do automatically\n  /// when this import is properly removed.\n  import wasi:cli/stdin@0.2.0-rc-2023-12-05;\n\n  /// This is the default handler to use when user code simply wants to make an\n  /// HTTP request (e.g., via `fetch()`).\n  import outgoing-handler;\n\n  /// The host delivers incoming HTTP requests to a component by calling the\n  /// `handle` function of this exported interface. A host may arbitrarily reuse\n  /// or not reuse component instance when delivering incoming HTTP requests and\n  /// thus a component must be able to handle 0..N calls to `handle`.\n  export incoming-handler;\n}\n" ;
const _ : & str = "/// This interface defines all of the types and methods for implementing\n/// HTTP Requests and Responses, both incoming and outgoing, as well as\n/// their headers, trailers, and bodies.\ninterface types {\n  use wasi:clocks/monotonic-clock@0.2.0-rc-2023-11-10.{duration};\n  use wasi:io/streams@0.2.0-rc-2023-11-10.{input-stream, output-stream};\n  use wasi:io/error@0.2.0-rc-2023-11-10.{error as io-error};\n  use wasi:io/poll@0.2.0-rc-2023-11-10.{pollable};\n\n  /// This type corresponds to HTTP standard Methods.\n  variant method {\n    get,\n    head,\n    post,\n    put,\n    delete,\n    connect,\n    options,\n    trace,\n    patch,\n    other(string)\n  }\n\n  /// This type corresponds to HTTP standard Related Schemes.\n  variant scheme {\n    HTTP,\n    HTTPS,\n    other(string)\n  }\n\n  /// These cases are inspired by the IANA HTTP Proxy Error Types:\n  ///   https://www.iana.org/assignments/http-proxy-status/http-proxy-status.xhtml#table-http-proxy-error-types\n  variant error-code {\n    DNS-timeout,\n    DNS-error(DNS-error-payload),\n    destination-not-found,\n    destination-unavailable,\n    destination-IP-prohibited,\n    destination-IP-unroutable,\n    connection-refused,\n    connection-terminated,\n    connection-timeout,\n    connection-read-timeout,\n    connection-write-timeout,\n    connection-limit-reached,\n    TLS-protocol-error,\n    TLS-certificate-error,\n    TLS-alert-received(TLS-alert-received-payload),\n    HTTP-request-denied,\n    HTTP-request-length-required,\n    HTTP-request-body-size(option<u64>),\n    HTTP-request-method-invalid,\n    HTTP-request-URI-invalid,\n    HTTP-request-URI-too-long,\n    HTTP-request-header-section-size(option<u32>),\n    HTTP-request-header-size(option<field-size-payload>),\n    HTTP-request-trailer-section-size(option<u32>),\n    HTTP-request-trailer-size(field-size-payload),\n    HTTP-response-incomplete,\n    HTTP-response-header-section-size(option<u32>),\n    HTTP-response-header-size(field-size-payload),\n    HTTP-response-body-size(option<u64>),\n    HTTP-response-trailer-section-size(option<u32>),\n    HTTP-response-trailer-size(field-size-payload),\n    HTTP-response-transfer-coding(option<string>),\n    HTTP-response-content-coding(option<string>),\n    HTTP-response-timeout,\n    HTTP-upgrade-failed,\n    HTTP-protocol-error,\n    loop-detected,\n    configuration-error,\n    /// This is a catch-all error for anything that doesn\'t fit cleanly into a\n    /// more specific case. It also includes an optional string for an\n    /// unstructured description of the error. Users should not depend on the\n    /// string for diagnosing errors, as it\'s not required to be consistent\n    /// between implementations.\n    internal-error(option<string>)\n  }\n\n  /// Defines the case payload type for `DNS-error` above:\n  record DNS-error-payload {\n    rcode: option<string>,\n    info-code: option<u16>\n  }\n\n  /// Defines the case payload type for `TLS-alert-received` above:\n  record TLS-alert-received-payload {\n    alert-id: option<u8>,\n    alert-message: option<string>\n  }\n\n  /// Defines the case payload type for `HTTP-response-{header,trailer}-size` above:\n  record field-size-payload {\n    field-name: option<string>,\n    field-size: option<u32>\n  }\n\n  /// Attempts to extract a http-related `error` from the wasi:io `error`\n  /// provided.\n  ///\n  /// Stream operations which return\n  /// `wasi:io/stream/stream-error::last-operation-failed` have a payload of\n  /// type `wasi:io/error/error` with more information about the operation\n  /// that failed. This payload can be passed through to this function to see\n  /// if there\'s http-related information about the error to return.\n  ///\n  /// Note that this function is fallible because not all io-errors are\n  /// http-related errors.\n  http-error-code: func(err: borrow<io-error>) -> option<error-code>;\n\n  /// This type enumerates the different kinds of errors that may occur when\n  /// setting or appending to a `fields` resource.\n  variant header-error {\n    /// This error indicates that a `field-key` or `field-value` was\n    /// syntactically invalid when used with an operation that sets headers in a\n    /// `fields`.\n    invalid-syntax,\n\n    /// This error indicates that a forbidden `field-key` was used when trying\n    /// to set a header in a `fields`.\n    forbidden,\n\n    /// This error indicates that the operation on the `fields` was not\n    /// permitted because the fields are immutable.\n    immutable,\n  }\n\n  /// Field keys are always strings.\n  type field-key = string;\n\n  /// Field values should always be ASCII strings. However, in\n  /// reality, HTTP implementations often have to interpret malformed values,\n  /// so they are provided as a list of bytes.\n  type field-value = list<u8>;\n\n  /// This following block defines the `fields` resource which corresponds to\n  /// HTTP standard Fields. Fields are a common representation used for both\n  /// Headers and Trailers.\n  ///\n  /// A `fields` may be mutable or immutable. A `fields` created using the\n  /// constructor, `from-list`, or `clone` will be mutable, but a `fields`\n  /// resource given by other means (including, but not limited to,\n  /// `incoming-request.headers`, `outgoing-request.headers`) might be be\n  /// immutable. In an immutable fields, the `set`, `append`, and `delete`\n  /// operations will fail with `header-error.immutable`.\n  resource fields {\n\n    /// Construct an empty HTTP Fields.\n    ///\n    /// The resulting `fields` is mutable.\n    constructor();\n\n    /// Construct an HTTP Fields.\n    ///\n    /// The resulting `fields` is mutable.\n    ///\n    /// The list represents each key-value pair in the Fields. Keys\n    /// which have multiple values are represented by multiple entries in this\n    /// list with the same key.\n    ///\n    /// The tuple is a pair of the field key, represented as a string, and\n    /// Value, represented as a list of bytes. In a valid Fields, all keys\n    /// and values are valid UTF-8 strings. However, values are not always\n    /// well-formed, so they are represented as a raw list of bytes.\n    ///\n    /// An error result will be returned if any header or value was\n    /// syntactically invalid, or if a header was forbidden.\n    from-list: static func(\n      entries: list<tuple<field-key,field-value>>\n    ) -> result<fields, header-error>;\n\n    /// Get all of the values corresponding to a key. If the key is not present\n    /// in this `fields`, an empty list is returned. However, if the key is\n    /// present but empty, this is represented by a list with one or more\n    /// empty field-values present.\n    get: func(name: field-key) -> list<field-value>;\n\n    /// Returns `true` when the key is present in this `fields`. If the key is\n    /// syntactically invalid, `false` is returned.\n    has: func(name: field-key) -> bool;\n\n    /// Set all of the values for a key. Clears any existing values for that\n    /// key, if they have been set.\n    ///\n    /// Fails with `header-error.immutable` if the `fields` are immutable.\n    set: func(name: field-key, value: list<field-value>) -> result<_, header-error>;\n\n    /// Delete all values for a key. Does nothing if no values for the key\n    /// exist.\n    ///\n    /// Fails with `header-error.immutable` if the `fields` are immutable.\n    delete: func(name: field-key) -> result<_, header-error>;\n\n    /// Append a value for a key. Does not change or delete any existing\n    /// values for that key.\n    ///\n    /// Fails with `header-error.immutable` if the `fields` are immutable.\n    append: func(name: field-key, value: field-value) -> result<_, header-error>;\n\n    /// Retrieve the full set of keys and values in the Fields. Like the\n    /// constructor, the list represents each key-value pair.\n    ///\n    /// The outer list represents each key-value pair in the Fields. Keys\n    /// which have multiple values are represented by multiple entries in this\n    /// list with the same key.\n    entries: func() -> list<tuple<field-key,field-value>>;\n\n    /// Make a deep copy of the Fields. Equivelant in behavior to calling the\n    /// `fields` constructor on the return value of `entries`. The resulting\n    /// `fields` is mutable.\n    clone: func() -> fields;\n  }\n\n  /// Headers is an alias for Fields.\n  type headers = fields;\n\n  /// Trailers is an alias for Fields.\n  type trailers = fields;\n\n  /// Represents an incoming HTTP Request.\n  resource incoming-request {\n\n    /// Returns the method of the incoming request.\n    method: func() -> method;\n\n    /// Returns the path with query parameters from the request, as a string.\n    path-with-query: func() -> option<string>;\n\n    /// Returns the protocol scheme from the request.\n    scheme: func() -> option<scheme>;\n\n    /// Returns the authority from the request, if it was present.\n    authority: func() -> option<string>;\n\n    /// Get the `headers` associated with the request.\n    ///\n    /// The returned `headers` resource is immutable: `set`, `append`, and\n    /// `delete` operations will fail with `header-error.immutable`.\n    ///\n    /// The `headers` returned are a child resource: it must be dropped before\n    /// the parent `incoming-request` is dropped. Dropping this\n    /// `incoming-request` before all children are dropped will trap.\n    headers: func() -> headers;\n\n    /// Gives the `incoming-body` associated with this request. Will only\n    /// return success at most once, and subsequent calls will return error.\n    consume: func() -> result<incoming-body>;\n  }\n\n  /// Represents an outgoing HTTP Request.\n  resource outgoing-request {\n\n    /// Construct a new `outgoing-request` with a default `method` of `GET`, and\n    /// `none` values for `path-with-query`, `scheme`, and `authority`.\n    ///\n    /// * `headers` is the HTTP Headers for the Request.\n    ///\n    /// It is possible to construct, or manipulate with the accessor functions\n    /// below, an `outgoing-request` with an invalid combination of `scheme`\n    /// and `authority`, or `headers` which are not permitted to be sent.\n    /// It is the obligation of the `outgoing-handler.handle` implementation\n    /// to reject invalid constructions of `outgoing-request`.\n    constructor(\n      headers: headers\n    );\n\n    /// Returns the resource corresponding to the outgoing Body for this\n    /// Request.\n    ///\n    /// Returns success on the first call: the `outgoing-body` resource for\n    /// this `outgoing-request` can be retrieved at most once. Subsequent\n    /// calls will return error.\n    body: func() -> result<outgoing-body>;\n\n    /// Get the Method for the Request.\n    method: func() -> method;\n    /// Set the Method for the Request. Fails if the string present in a\n    /// `method.other` argument is not a syntactically valid method.\n    set-method: func(method: method) -> result;\n\n    /// Get the combination of the HTTP Path and Query for the Request.\n    /// When `none`, this represents an empty Path and empty Query.\n    path-with-query: func() -> option<string>;\n    /// Set the combination of the HTTP Path and Query for the Request.\n    /// When `none`, this represents an empty Path and empty Query. Fails is the\n    /// string given is not a syntactically valid path and query uri component.\n    set-path-with-query: func(path-with-query: option<string>) -> result;\n\n    /// Get the HTTP Related Scheme for the Request. When `none`, the\n    /// implementation may choose an appropriate default scheme.\n    scheme: func() -> option<scheme>;\n    /// Set the HTTP Related Scheme for the Request. When `none`, the\n    /// implementation may choose an appropriate default scheme. Fails if the\n    /// string given is not a syntactically valid uri scheme.\n    set-scheme: func(scheme: option<scheme>) -> result;\n\n    /// Get the HTTP Authority for the Request. A value of `none` may be used\n    /// with Related Schemes which do not require an Authority. The HTTP and\n    /// HTTPS schemes always require an authority.\n    authority: func() -> option<string>;\n    /// Set the HTTP Authority for the Request. A value of `none` may be used\n    /// with Related Schemes which do not require an Authority. The HTTP and\n    /// HTTPS schemes always require an authority. Fails if the string given is\n    /// not a syntactically valid uri authority.\n    set-authority: func(authority: option<string>) -> result;\n\n    /// Get the headers associated with the Request.\n    ///\n    /// The returned `headers` resource is immutable: `set`, `append`, and\n    /// `delete` operations will fail with `header-error.immutable`.\n    ///\n    /// This headers resource is a child: it must be dropped before the parent\n    /// `outgoing-request` is dropped, or its ownership is transfered to\n    /// another component by e.g. `outgoing-handler.handle`.\n    headers: func() -> headers;\n  }\n\n  /// Parameters for making an HTTP Request. Each of these parameters is\n  /// currently an optional timeout applicable to the transport layer of the\n  /// HTTP protocol.\n  ///\n  /// These timeouts are separate from any the user may use to bound a\n  /// blocking call to `wasi:io/poll.poll`.\n  resource request-options {\n    /// Construct a default `request-options` value.\n    constructor();\n\n    /// The timeout for the initial connect to the HTTP Server.\n    connect-timeout: func() -> option<duration>;\n\n    /// Set the timeout for the initial connect to the HTTP Server. An error\n    /// return value indicates that this timeout is not supported.\n    set-connect-timeout: func(duration: option<duration>) -> result;\n\n    /// The timeout for receiving the first byte of the Response body.\n    first-byte-timeout: func() -> option<duration>;\n\n    /// Set the timeout for receiving the first byte of the Response body. An\n    /// error return value indicates that this timeout is not supported.\n    set-first-byte-timeout: func(duration: option<duration>) -> result;\n\n    /// The timeout for receiving subsequent chunks of bytes in the Response\n    /// body stream.\n    between-bytes-timeout: func() -> option<duration>;\n\n    /// Set the timeout for receiving subsequent chunks of bytes in the Response\n    /// body stream. An error return value indicates that this timeout is not\n    /// supported.\n    set-between-bytes-timeout: func(duration: option<duration>) -> result;\n  }\n\n  /// Represents the ability to send an HTTP Response.\n  ///\n  /// This resource is used by the `wasi:http/incoming-handler` interface to\n  /// allow a Response to be sent corresponding to the Request provided as the\n  /// other argument to `incoming-handler.handle`.\n  resource response-outparam {\n\n    /// Set the value of the `response-outparam` to either send a response,\n    /// or indicate an error.\n    ///\n    /// This method consumes the `response-outparam` to ensure that it is\n    /// called at most once. If it is never called, the implementation\n    /// will respond with an error.\n    ///\n    /// The user may provide an `error` to `response` to allow the\n    /// implementation determine how to respond with an HTTP error response.\n    set: static func(\n      param: response-outparam,\n      response: result<outgoing-response, error-code>,\n    );\n  }\n\n  /// This type corresponds to the HTTP standard Status Code.\n  type status-code = u16;\n\n  /// Represents an incoming HTTP Response.\n  resource incoming-response {\n\n    /// Returns the status code from the incoming response.\n    status: func() -> status-code;\n\n    /// Returns the headers from the incoming response.\n    ///\n    /// The returned `headers` resource is immutable: `set`, `append`, and\n    /// `delete` operations will fail with `header-error.immutable`.\n    ///\n    /// This headers resource is a child: it must be dropped before the parent\n    /// `incoming-response` is dropped.\n    headers: func() -> headers;\n\n    /// Returns the incoming body. May be called at most once. Returns error\n    /// if called additional times.\n    consume: func() -> result<incoming-body>;\n  }\n\n  /// Represents an incoming HTTP Request or Response\'s Body.\n  ///\n  /// A body has both its contents - a stream of bytes - and a (possibly\n  /// empty) set of trailers, indicating that the full contents of the\n  /// body have been received. This resource represents the contents as\n  /// an `input-stream` and the delivery of trailers as a `future-trailers`,\n  /// and ensures that the user of this interface may only be consuming either\n  /// the body contents or waiting on trailers at any given time.\n  resource incoming-body {\n\n    /// Returns the contents of the body, as a stream of bytes.\n    ///\n    /// Returns success on first call: the stream representing the contents\n    /// can be retrieved at most once. Subsequent calls will return error.\n    ///\n    /// The returned `input-stream` resource is a child: it must be dropped\n    /// before the parent `incoming-body` is dropped, or consumed by\n    /// `incoming-body.finish`.\n    ///\n    /// This invariant ensures that the implementation can determine whether\n    /// the user is consuming the contents of the body, waiting on the\n    /// `future-trailers` to be ready, or neither. This allows for network\n    /// backpressure is to be applied when the user is consuming the body,\n    /// and for that backpressure to not inhibit delivery of the trailers if\n    /// the user does not read the entire body.\n    %stream: func() -> result<input-stream>;\n\n    /// Takes ownership of `incoming-body`, and returns a `future-trailers`.\n    /// This function will trap if the `input-stream` child is still alive.\n    finish: static func(this: incoming-body) -> future-trailers;\n  }\n\n  /// Represents a future which may eventaully return trailers, or an error.\n  ///\n  /// In the case that the incoming HTTP Request or Response did not have any\n  /// trailers, this future will resolve to the empty set of trailers once the\n  /// complete Request or Response body has been received.\n  resource future-trailers {\n\n    /// Returns a pollable which becomes ready when either the trailers have\n    /// been received, or an error has occured. When this pollable is ready,\n    /// the `get` method will return `some`.\n    subscribe: func() -> pollable;\n\n    /// Returns the contents of the trailers, or an error which occured,\n    /// once the future is ready.\n    ///\n    /// The outer `option` represents future readiness. Users can wait on this\n    /// `option` to become `some` using the `subscribe` method.\n    ///\n    /// The outer `result` is used to retrieve the trailers or error at most\n    /// once. It will be success on the first call in which the outer option\n    /// is `some`, and error on subsequent calls.\n    ///\n    /// The inner `result` represents that either the HTTP Request or Response\n    /// body, as well as any trailers, were received successfully, or that an\n    /// error occured receiving them. The optional `trailers` indicates whether\n    /// or not trailers were present in the body.\n    ///\n    /// When some `trailers` are returned by this method, the `trailers`\n    /// resource is immutable, and a child. Use of the `set`, `append`, or\n    /// `delete` methods will return an error, and the resource must be\n    /// dropped before the parent `future-trailers` is dropped.\n    get: func() -> option<result<result<option<trailers>, error-code>>>;\n  }\n\n  /// Represents an outgoing HTTP Response.\n  resource outgoing-response {\n\n    /// Construct an `outgoing-response`, with a default `status-code` of `200`.\n    /// If a different `status-code` is needed, it must be set via the\n    /// `set-status-code` method.\n    ///\n    /// * `headers` is the HTTP Headers for the Response.\n    constructor(headers: headers);\n\n    /// Get the HTTP Status Code for the Response.\n    status-code: func() -> status-code;\n\n    /// Set the HTTP Status Code for the Response. Fails if the status-code\n    /// given is not a valid http status code.\n    set-status-code: func(status-code: status-code) -> result;\n\n    /// Get the headers associated with the Request.\n    ///\n    /// The returned `headers` resource is immutable: `set`, `append`, and\n    /// `delete` operations will fail with `header-error.immutable`.\n    ///\n    /// This headers resource is a child: it must be dropped before the parent\n    /// `outgoing-request` is dropped, or its ownership is transfered to\n    /// another component by e.g. `outgoing-handler.handle`.\n    headers: func() -> headers;\n\n    /// Returns the resource corresponding to the outgoing Body for this Response.\n    ///\n    /// Returns success on the first call: the `outgoing-body` resource for\n    /// this `outgoing-response` can be retrieved at most once. Subsequent\n    /// calls will return error.\n    body: func() -> result<outgoing-body>;\n  }\n\n  /// Represents an outgoing HTTP Request or Response\'s Body.\n  ///\n  /// A body has both its contents - a stream of bytes - and a (possibly\n  /// empty) set of trailers, inducating the full contents of the body\n  /// have been sent. This resource represents the contents as an\n  /// `output-stream` child resource, and the completion of the body (with\n  /// optional trailers) with a static function that consumes the\n  /// `outgoing-body` resource, and ensures that the user of this interface\n  /// may not write to the body contents after the body has been finished.\n  ///\n  /// If the user code drops this resource, as opposed to calling the static\n  /// method `finish`, the implementation should treat the body as incomplete,\n  /// and that an error has occured. The implementation should propogate this\n  /// error to the HTTP protocol by whatever means it has available,\n  /// including: corrupting the body on the wire, aborting the associated\n  /// Request, or sending a late status code for the Response.\n  resource outgoing-body {\n\n    /// Returns a stream for writing the body contents.\n    ///\n    /// The returned `output-stream` is a child resource: it must be dropped\n    /// before the parent `outgoing-body` resource is dropped (or finished),\n    /// otherwise the `outgoing-body` drop or `finish` will trap.\n    ///\n    /// Returns success on the first call: the `output-stream` resource for\n    /// this `outgoing-body` may be retrieved at most once. Subsequent calls\n    /// will return error.\n    write: func() -> result<output-stream>;\n\n    /// Finalize an outgoing body, optionally providing trailers. This must be\n    /// called to signal that the response is complete. If the `outgoing-body`\n    /// is dropped without calling `outgoing-body.finalize`, the implementation\n    /// should treat the body as corrupted.\n    ///\n    /// Fails if the body\'s `outgoing-request` or `outgoing-response` was\n    /// constructed with a Content-Length header, and the contents written\n    /// to the body (via `write`) does not match the value given in the\n    /// Content-Length.\n    finish: static func(\n      this: outgoing-body,\n      trailers: option<trailers>\n    ) -> result<_, error-code>;\n  }\n\n  /// Represents a future which may eventaully return an incoming HTTP\n  /// Response, or an error.\n  ///\n  /// This resource is returned by the `wasi:http/outgoing-handler` interface to\n  /// provide the HTTP Response corresponding to the sent Request.\n  resource future-incoming-response {\n    /// Returns a pollable which becomes ready when either the Response has\n    /// been received, or an error has occured. When this pollable is ready,\n    /// the `get` method will return `some`.\n    subscribe: func() -> pollable;\n\n    /// Returns the incoming HTTP Response, or an error, once one is ready.\n    ///\n    /// The outer `option` represents future readiness. Users can wait on this\n    /// `option` to become `some` using the `subscribe` method.\n    ///\n    /// The outer `result` is used to retrieve the response or error at most\n    /// once. It will be success on the first call in which the outer option\n    /// is `some`, and error on subsequent calls.\n    ///\n    /// The inner `result` represents that either the incoming HTTP Response\n    /// status and headers have recieved successfully, or that an error\n    /// occured. Errors may also occur while consuming the response body,\n    /// but those will be reported by the `incoming-body` and its\n    /// `output-stream` child.\n    get: func() -> option<result<result<incoming-response, error-code>>>;\n\n  }\n}\n" ;
const _ : & str = "package litehouse:plugin;\n\ninterface plugin {\n    record event {\n        id: u64,\n        timestamp: u64,\n        inner: update,\n    }\n\n    variant update {\n      time(u64),\n      temperature(float64),\n      wind-speed(float64),\n    }\n\n    variant subscription {\n      time(time-subscription),\n    }\n\n    variant time-subscription {\n      every(every),\n      at(u64),\n    }\n\n    record every {\n      amount: u64,\n      unit: time-unit,\n    }\n\n    variant time-unit {\n      second,\n      minute,\n      hour,\n      day,\n      week,\n      month,\n      year,\n    }\n\n    resource runner {\n      constructor();\n      subscribe: func() -> result<list<subscription>, u32>;\n      update: func(events: list<event>) -> result<bool, u32>;\n    }\n\n    generate-config-schema: func() -> option<string>;\n}\n\nworld plugin-host {\n  use plugin.{event, subscription};\n  import wasi:http/outgoing-handler@0.2.0-rc-2023-12-05;\n\n  import update: func(event: event);\n  export plugin;\n}" ;
impl Guest for TasmotaConfig {
    fn generate_config_schema() -> Option<String> {
        plugin::serde_json::to_string(
            &::schemars::gen::SchemaGenerator::default().into_root_schema_for::<TasmotaConfig>(),
        )
        .ok()
    }
}
pub struct TasmotaPlugin {
    state: Mutex<bool>,
}
pub struct TasmotaConfig;
const _: () = {
    #[automatically_derived]
    #[allow(unused_braces)]
    impl schemars::JsonSchema for TasmotaConfig {
        fn schema_name() -> std::string::String {
            "TasmotaConfig".to_owned()
        }
        fn schema_id() -> std::borrow::Cow<'static, str> {
            std::borrow::Cow::Borrowed("tasmota::TasmotaConfig")
        }
        fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
            gen.subschema_for::<()>()
        }
    };
};
impl TasmotaPlugin {
    fn get_state(&self) -> bool {
        *self.state.lock().unwrap()
    }
    fn set_state(&self, state: bool) {
        *self.state.lock().unwrap() = state;
    }
}
impl GuestRunner for TasmotaPlugin {
    fn new() -> Self {
        plugin::tracing_subscriber();
        Self {
            state: Mutex::new(false),
        }
    }
    fn subscribe(&self) -> Result<Vec<Subscription>, u32> {
        Ok(<[_]>::into_vec(
            #[rustc_box]
            ::alloc::boxed::Box::new([Subscription::Time(
                exports::litehouse::plugin::plugin::TimeSubscription::Every(Every {
                    amount: 1,
                    unit: TimeUnit::Second,
                }),
            )]),
        ))
    }
    fn update(&self, events: Vec<exports::litehouse::plugin::plugin::Event>) -> Result<bool, u32> {
        let state = self.get_state();
        let headers = Fields::new();
        let req = OutgoingRequest::new(headers);
        req.set_path_with_query(Some(&{
            let res = ::alloc::fmt::format(format_args!(
                "/cm?cmnd=Power%20{0}",
                if state { "OFF" } else { "ON" }
            ));
            res
        }))
        .expect("ok");
        req.set_authority(Some("192.168.1.71:80"));
        req.set_scheme(Some(&Scheme::Http));
        let opts = RequestOptions::new();
        let x = outgoing_handler::handle(req, Some(opts)).unwrap();
        x.subscribe().block();
        let resp = x.get().unwrap().unwrap().unwrap();
        let body = resp
            .consume()
            .unwrap()
            .stream()
            .unwrap()
            .blocking_read(1024)
            .unwrap();
        {
            ::std::io::_print(format_args!(
                "body: {0:?}\n",
                String::from_utf8(body).unwrap()
            ));
        };
        self.set_state(!state);
        Ok(true)
    }
}
