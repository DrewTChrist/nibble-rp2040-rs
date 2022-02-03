//use keyberon::action::Action::Trans;
use keyberon::action::{k, l, m, Action, Action::*};
use keyberon::key_code::KeyCode::*;
#[allow(unused_macros)]

// Shift + KeyCode
macro_rules! s {
    ($k:ident) => {
        m(&[LShift, $k])
    };
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CustomActions {
    Underglow,
    Bootloader,
}

const UNDERGLOW: Action<CustomActions> = Action::Custom(CustomActions::Underglow);
const BOOTLOADER: Action<CustomActions> = Action::Custom(CustomActions::Bootloader);
const COPY: Action<CustomActions> = m(&[LCtrl, C]);
const PASTE: Action<CustomActions> = m(&[LCtrl, V]);

#[rustfmt::skip]
pub static LAYERS: keyberon::layout::Layers<CustomActions> = &[
    /* QWERTY */
    &[
        &[k(No),   k(Escape),   k(Kb1),  k(Kb2),  k(Kb3),   k(Kb4),   k(Kb5),   k(Kb6),   k(Kb7),   k(Kb8),   k(Kb9),   k(Kb0),    k(Minus),    k(Equal),    k(BSpace), k(Home)],
        &[k(LGui), k(Tab),      k(Q),    k(W),    k(E),     k(R),     k(T),     k(Y),     k(U),     k(I),     k(O),     k(P),      k(LBracket), k(RBracket), k(Bslash), k(End)],
        &[l(1),    k(CapsLock), k(A),    k(S),    k(D),     k(F),     k(G),     k(H),     k(J),     k(K),     k(L),     k(SColon), k(Quote),    k(No),       k(Enter),  k(PgUp)],
        &[COPY,    k(LShift),   k(Z),    k(X),    k(C),     k(V),     k(B),     k(N),     k(M),     k(Comma), k(Dot),   k(Slash),  k(LShift),   k(No),       k(Up),     k(PgDown)],
        &[PASTE,   k(LCtrl),    k(LGui), k(LAlt), k(No),    k(No),    k(Space), k(No),    k(No),    k(RAlt),  k(RCtrl),  k(RGui),  k(Left),     k(No),       k(Down),   k(Right)],
    ], 
    /* misc layer */
    &[
        &[k(No),           Trans, k(F1), k(F2), k(F3), k(F4), k(F5), k(F6), k(F7), k(F8), k(F9), k(F10), k(F11), k(F12), Trans, UNDERGLOW],
        &[k(No),           Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans,  Trans,  Trans,  Trans, BOOTLOADER],
        &[k(No),           Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans,  Trans,  k(No),  Trans, Trans],
        &[DefaultLayer(0), Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans,  Trans,  k(No),  Trans, Trans],
        &[DefaultLayer(2), Trans, Trans, Trans, k(No), k(No), Trans, k(No), k(No), Trans, Trans, Trans,  Trans,  k(No),  Trans, Trans],
    ], 
    /* Dvorak */
    &[
        &[k(No),   k(Escape),   k(Kb1),    k(Kb2),   k(Kb3),   k(Kb4),   k(Kb5),   k(Kb6),   k(Kb7),   k(Kb8),   k(Kb9),   k(Kb0),  k(LBracket), k(RBracket), k(BSpace), k(Home)],
        &[k(LGui), k(Tab),      k(Quote),  k(Comma), k(Dot),   k(P),     k(Y),     k(F),     k(G),     k(C),     k(R),     k(L),    k(Slash),    k(Equal),    k(Bslash), k(End)],
        &[l(1),    k(CapsLock), k(A),      k(O),     k(E),     k(U),     k(I),     k(D),     k(H),     k(T),     k(N),     k(S),    k(Minus),    k(No),       k(Enter),  k(PgUp)],
        &[COPY,    k(LShift),   k(SColon), k(Q),     k(J),     k(K),     k(X),     k(B),     k(M),     k(W),     k(V),     k(Z),    k(LShift),   k(No),       k(Up),     k(PgDown)],
        &[PASTE,   k(LCtrl),    k(LGui),   k(LAlt),  k(No),    k(No),    k(Space), k(No),    k(No),    k(RAlt),  k(RCtrl), k(RGui), k(Left),     k(No),       k(Down),   k(Right)],
    ], 
];
