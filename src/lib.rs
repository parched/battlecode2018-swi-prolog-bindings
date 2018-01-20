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

extern crate battlecode_engine;
#[macro_use]
extern crate c_str_macro;
#[macro_use]
extern crate lazy_static;
extern crate swipl_sys;

use swipl_sys::{control_t, foreign_t, term_t, PL_copy_term_ref, PL_foreign_context,
                PL_foreign_context_address, PL_foreign_control, PL_get_atom_chars,
                PL_new_term_ref, PL_register_foreign, PL_unify_atom_chars, PL_unify_integer,
                PL_unify_list, PL_unify_nil, PL_warning, _PL_retry, _PL_retry_address,
                PL_FA_NONDETERMINISTIC, PL_FIRST_CALL, PL_PRUNED, PL_REDO};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::os::raw::c_void;

use battlecode_engine::controller::GameController;
use battlecode_engine::location::Planet;
use battlecode_engine::unit::Unit;

use std::sync::Mutex;

lazy_static! {
    static ref GAME_CONTROLLER: Mutex<GameController> = Mutex::new(GameController::new_player_env().unwrap());
}

extern "C" fn lowercase(mixed: term_t, lower: term_t) -> foreign_t {
    unsafe {
        let mut chars: *mut c_char = std::mem::uninitialized();

        if PL_get_atom_chars(mixed, &mut chars as *mut *mut c_char) == 0 {
            return PL_warning(c_str!("lowercase/2: instantiation fault").as_ptr()) as foreign_t;
        }

        let lower_of_mixed = CStr::from_ptr(chars).to_str().unwrap().to_lowercase();

        PL_unify_atom_chars(lower, CString::new(lower_of_mixed).unwrap().as_ptr()) as foreign_t
    }
}

fn get_atom_chars(term: term_t) -> Result<&'static str, foreign_t> {
    unsafe {
        let mut chars: *mut c_char = std::mem::uninitialized();

        if PL_get_atom_chars(term, &mut chars as *mut *mut c_char) == 0 {
            Err(PL_warning(c_str!("lowercase/2: instantiation fault").as_ptr()) as foreign_t)
        } else {
            Ok(CStr::from_ptr(chars).to_str().unwrap()) // TODO: unwrap
        }
    }
}

fn unify_atom_chars(atom: term_t, chars: &str) -> foreign_t {
    unsafe { PL_unify_atom_chars(atom, CString::new(chars).unwrap().as_ptr()) as foreign_t }
}

fn unify_integer_list<T: Iterator<Item = isize>>(list: term_t, iter: T) -> foreign_t {
    unsafe {
        let l = PL_copy_term_ref(list);
        let a = PL_new_term_ref();

        for integer in iter {
            if PL_unify_list(l, a, l) == 0 || PL_unify_integer(a, integer) == 0 {
                return false as foreign_t;
            }
        }

        PL_unify_nil(l) as foreign_t
    }
}

extern "C" fn unit_ids(list: term_t) -> foreign_t {
    let gc = GAME_CONTROLLER.lock().unwrap();
    let unit_ids = gc.units_ref();
    let unit_ids_iter = unit_ids.iter().map(|unit| unit.id() as isize);
    unify_integer_list(list, unit_ids_iter)
}

extern "C" fn planet(atom: term_t) -> foreign_t {
    let planet = match GAME_CONTROLLER.lock().unwrap().planet() {
        Planet::Earth => "earth",
        Planet::Mars => "mars",
    };

    unify_atom_chars(atom, planet)
}

extern "C" fn next_turn() -> foreign_t {
    GAME_CONTROLLER.lock().unwrap().next_turn();
    true as foreign_t
}

extern "C" fn is_move_ready_list(list: term_t) -> foreign_t {
    let gc = GAME_CONTROLLER.lock().unwrap();
    let unit_ids = gc.units_ref();
    let unit_ids_iter = unit_ids
        .iter()
        .filter(|unit| gc.is_move_ready(unit.id()))
        .map(|unit| unit.id() as isize);
    unify_integer_list(list, unit_ids_iter)
}

extern "C" fn is_move_ready(id: term_t, handle: control_t) -> foreign_t {
    unsafe {
        let mut iter = match PL_foreign_control(handle) as u32 {
            PL_FIRST_CALL => {
                let gc = GAME_CONTROLLER.lock().unwrap();
                // TODO: check if term is variable first
                // and short circuit by PL_get_integer
                
                // Create an iterator and box it

                let unit_ids = gc.units_ref()
                    .iter()
                    .map(|unit| unit.id())
                    .filter(|id| gc.is_move_ready(*id))
                    .map(|id| id as isize)
                    .collect::<Vec<_>>();
                // let unit_ids = vec![4 as isize, 6 as isize];

                Box::new(unit_ids.into_iter())
            }
            PL_REDO => {
                // Reconstruct the boxed iterator
                Box::from_raw(PL_foreign_context_address(handle)
                    as *mut std::vec::IntoIter<isize>)
            }
            PL_PRUNED => {
                // Just construct the Box and drop it
                //
                Box::from_raw(PL_foreign_context_address(handle)
                    as *mut std::vec::IntoIter<isize>);
                return true as foreign_t;
            }
            _ => unreachable!(),
        };

        let mut success = false;
        for id_candidate in &mut iter {
            if PL_unify_integer(id, id_candidate) != 0 {
                success = true;
                break;
            }
        }

        if success {
            _PL_retry_address(Box::into_raw(iter) as *mut c_void) as foreign_t
        } else {
            false as foreign_t
        }
    }
}

#[no_mangle]
pub extern "C" fn install() {
    unsafe {
        PL_register_foreign(
            c_str!("lowercase").as_ptr(),
            2,
            Some(std::mem::transmute(
                lowercase as extern "C" fn(term_t, term_t) -> foreign_t,
            )),
            0,
        );
        PL_register_foreign(
            c_str!("planet").as_ptr(),
            1,
            Some(std::mem::transmute(
                planet as extern "C" fn(term_t) -> foreign_t,
            )),
            0,
        );
        PL_register_foreign(
            c_str!("unit_ids").as_ptr(),
            1,
            Some(std::mem::transmute(
                unit_ids as extern "C" fn(term_t) -> foreign_t,
            )),
            0,
        );
        PL_register_foreign(
            c_str!("next_turn").as_ptr(),
            0,
            Some(std::mem::transmute(
                next_turn as extern "C" fn() -> foreign_t,
            )),
            0,
        );
        PL_register_foreign(
            c_str!("is_move_ready_list").as_ptr(),
            1,
            Some(std::mem::transmute(
                is_move_ready_list as extern "C" fn(term_t) -> foreign_t,
            )),
            0,
        );
        PL_register_foreign(
            c_str!("is_move_ready").as_ptr(),
            1,
            Some(std::mem::transmute(
                is_move_ready as extern "C" fn(term_t, control_t) -> foreign_t,
            )),
            PL_FA_NONDETERMINISTIC as i32,
        );
    }
}
