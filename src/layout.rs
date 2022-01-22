use keyberon::action::Action::Trans;
use keyberon::action::{k, l, m};
use keyberon::key_code::KeyCode::*;
#[allow(unused_macros)]

// Shift + KeyCode
macro_rules! s {
    ($k:ident) => {
        m(&[LShift, $k])
    };
}

#[rustfmt::skip]
pub static LAYERS: keyberon::layout::Layers = &[
    &[
        &[k(No), k(Escape),   k(Kb1),  k(Kb2),  k(Kb3),   k(Kb4),   k(Kb5),   k(Kb6),   k(Kb7),   k(Kb8),   k(Kb9),   k(Kb0),    k(Minus),    k(Equal),    k(BSpace), k(Application)],
        &[k(LGui), k(Tab),      k(Q),    k(W),    k(E),     k(R),     k(T),     k(Y),     k(U),     k(I),     k(O),     k(P),      k(LBracket), k(RBracket), k(Bslash), k(Delete)],
        &[k(Mute), k(CapsLock), k(A),    k(S),    k(D),     k(F),     k(G),     k(H),     k(J),     k(K),     k(L),     k(SColon), k(Quote),    k(No),       k(Enter),  k(PgUp)],
        &[k(VolUp), k(LShift),   k(Z),    k(X),    k(C),     k(V),     k(B),     k(N),     k(M),     k(Comma), k(Dot),   k(Slash),  k(LShift),   k(No),       k(Up),     k(PgDown)],
        &[k(VolDown), k(LCtrl),    k(LGui), k(LAlt), k(No),    k(No),    k(Space), k(No),    k(No),    k(RAlt),  k(RCtrl),  k(RGui),  k(Left),     k(No),       k(Down),   k(Right)],
    ], 
];
