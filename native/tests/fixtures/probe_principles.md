# CS principles — the enforceable canon, described in prose (machine-global corpus)

## dead_code_after_return [medium]
Never write code after a return statement. A statement that follows a return is unreachable dead
code: control has already left the function, so the code can never execute. Remove the unreachable
statements, or move them before the return.

## swallowed_error [high]
Never ignore, discard, or swallow an error. An error result that is thrown away — assigned to a
throwaway, matched to an empty handler, or unwrapped without care — hides a real failure and
becomes a silent hidden bug. Handle the error, or propagate it to the caller.

## unwrap_on_fallible [high]
Never unwrap or expect the result of a fallible call. Unwrapping a result that can fail forces a
panic on the error path instead of handling it. Match the result, use the question-mark operator,
or return the error to the caller.

## god_function [medium]
Never write an enormous function that does too many things. A function whose body runs on for
dozens of statements has too many responsibilities and is impossible to read, test, or reuse.
Split the long function into small single-responsibility units.

## undocumented_public_item [medium]
Never expose a public function or type without a documentation comment. A public item is an API
other code depends on; an undocumented public item leaves every caller guessing its contract.
Document every public item with a comment describing what it does.

## magic_number [medium]
Never bury an unexplained magic number literal in the code. A bare numeric constant with no name
hides its meaning and scatters the same value across the code. Give the magic number a named
constant that explains what the value means.

## non_descriptive_name [medium]
Never name a variable with a single meaningless letter. A single-letter or non-descriptive
identifier tells the reader nothing about what the value holds. Choose a clear descriptive name
that says what the binding represents.

## hardcoded_secret [high]
Never hardcode a secret in the source. A password, API key, token, or credential written as a
literal string in the code is a leaked secret the moment the code is shared. Read the secret from
the environment or a configuration store, never from a hardcoded literal.

## shell_injection [high]
Never interpolate untrusted input into a shell command string. Building a command from user input
and handing it to a shell to execute is a command injection vulnerability: the input can inject
arbitrary commands. Pass arguments as a list to the process directly, never through a shell.

## duplicated_code [medium]
Never duplicate the same code in two places. Copied and pasted logic that repeats a near-identical
block violates the do-not-repeat-yourself principle and drifts out of sync when only one copy is
fixed. Extract the duplicated code into one shared function.
