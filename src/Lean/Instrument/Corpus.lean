module

prelude
public import Init.Data.Option.Basic
public import Lean.Data.Options
public import Lean.Data.Name

public section

namespace Lean.Instrument.Corpus

register_builtin_option instrument.corpus : Bool := {
  defValue := false
  descr    := "If true, instrumented entrypoints serialize (input, state_in, output, state_out, message_delta) tuples to per-function corpus files for replay against the Rust port."
}

@[inline] def withCorpusCapture {m : Type → Type u} [Monad m] (_key : Name) (f : α → m β) (a : α) : m β :=
  f a

end Lean.Instrument.Corpus
