//   Copyright 2018 James Duley
//
//   Licensed under the Apache License, Version 2.0 (the "License");
//   you may not use this file except in compliance with the License.
//   You may obtain a copy of the License at
//
//       http://www.apache.org/licenses/LICENSE-2.0
//
//   Unless required by applicable law or agreed to in writing, software
//   distributed under the License is distributed on an "AS IS" BASIS,
//   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//   See the License for the specific language governing permissions and
//   limitations under the License.

#[macro_use]
extern crate c_str_macro;
extern crate swipl_sys;

use swipl_sys::{foreign_t, term_t, PL_get_atom_chars, PL_register_foreign, PL_unify_atom_chars,
                PL_warning};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

extern "C" fn pl_lowercase(mixed: term_t, lower: term_t) -> foreign_t {
    unsafe {
        let mut chars: *mut c_char = std::mem::uninitialized();

        if PL_get_atom_chars(mixed, &mut chars as *mut *mut c_char) == 0 {
            return PL_warning(c_str!("lowercase/2: instantiation fault").as_ptr()) as foreign_t;
        }

        let lower_of_mixed = CStr::from_ptr(chars).to_str().unwrap().to_lowercase();

        PL_unify_atom_chars(lower, CString::new(lower_of_mixed).unwrap().as_ptr()) as foreign_t
    }
}

#[no_mangle]
pub extern "C" fn install() {
    unsafe {
        PL_register_foreign(
            c_str!("lowercase").as_ptr(),
            2,
            Some(std::mem::transmute(
                pl_lowercase as extern "C" fn(term_t, term_t) -> foreign_t,
            )),
            0,
        );
    }
}
