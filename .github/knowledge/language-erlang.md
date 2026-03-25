# Erlang Best Practices

## Erlang Philosophy

Erlang was built at Ericsson for telecom switches — systems that must never go down. It runs on the BEAM virtual machine and provides lightweight processes, message passing, supervision trees, and hot code reloading. "Let it crash" is the core design principle.

- **Let it crash**: Don't write defensive code. Let processes fail and supervisors restart them.
- **Share nothing**: Processes communicate only via message passing. No shared memory.
- **Fault tolerance by design**: Supervision trees isolate failures. The system self-heals.

## Processes and Message Passing

```erlang
%% Spawn a process
Pid = spawn(fun() -> loop(State) end),

%% Send a message
Pid ! {hello, "world"},

%% Receive messages (selective)
receive
    {hello, Name} ->
        io:format("Hello, ~s~n", [Name]);
    {goodbye, Name} ->
        io:format("Goodbye, ~s~n", [Name])
after 5000 ->
    io:format("Timeout~n")
end.

%% A process loop
loop(State) ->
    receive
        {get, From} ->
            From ! {ok, State},
            loop(State);
        {set, NewState} ->
            loop(NewState);
        stop ->
            ok  % process exits
    end.
```

## Pattern Matching

```erlang
%% Function clauses
factorial(0) -> 1;
factorial(N) when N > 0 -> N * factorial(N - 1).

%% Tuple matching
handle({ok, Value}) -> process(Value);
handle({error, Reason}) -> log_error(Reason).

%% List patterns
sum([]) -> 0;
sum([H | T]) -> H + sum(T).

%% Map patterns (OTP 17+)
process_user(#{name := Name, age := Age}) when Age >= 18 ->
    {ok, Name};
process_user(#{age := Age}) ->
    {error, {too_young, Age}}.

%% Binary pattern matching (Erlang's killer feature for protocols)
parse_header(<<Version:8, Type:8, Length:16, Payload/binary>>) ->
    {Version, Type, Length, Payload}.
```

## OTP Behaviors

```erlang
%% gen_server — the workhorse
-module(counter).
-behaviour(gen_server).
-export([start_link/1, increment/1, get_value/1]).
-export([init/1, handle_call/3, handle_cast/2]).

start_link(InitialValue) ->
    gen_server:start_link({local, ?MODULE}, ?MODULE, InitialValue, []).

increment(Pid) ->
    gen_server:cast(Pid, increment).

get_value(Pid) ->
    gen_server:call(Pid, get_value).

%% Callbacks
init(InitialValue) ->
    {ok, #{count => InitialValue}}.

handle_call(get_value, _From, #{count := Count} = State) ->
    {reply, Count, State}.

handle_cast(increment, #{count := Count} = State) ->
    {noreply, State#{count := Count + 1}}.
```

## Supervision Trees

```erlang
%% Supervisor
-module(my_sup).
-behaviour(supervisor).
-export([start_link/0, init/1]).

start_link() ->
    supervisor:start_link({local, ?MODULE}, ?MODULE, []).

init([]) ->
    SupFlags = #{
        strategy => one_for_one,   % restart only the crashed child
        intensity => 5,            % max 5 restarts
        period => 60               % in 60 seconds
    },
    Children = [
        #{id => worker_1,
          start => {worker_mod, start_link, [arg1]},
          restart => permanent,
          type => worker},
        #{id => worker_2,
          start => {another_mod, start_link, []},
          restart => transient,
          type => worker}
    ],
    {ok, {SupFlags, Children}}.

%% Strategies:
%% one_for_one  — restart only the failed child
%% one_for_all  — restart all children
%% rest_for_one — restart failed child + those started after it
```

## Binary Processing

```erlang
%% Erlang excels at binary protocol parsing
%% TCP packet: [2-byte length][payload]
decode_packet(<<Length:16/big, Payload:Length/binary, Rest/binary>>) ->
    {ok, Payload, Rest};
decode_packet(Data) ->
    {incomplete, Data}.

%% Build binaries
encode_message(Type, Payload) ->
    Length = byte_size(Payload),
    <<Type:8, Length:16/big, Payload/binary>>.

%% Bit-level matching
decode_flags(<<ReadOnly:1, Hidden:1, System:1, _Reserved:5>>) ->
    #{readonly => ReadOnly =:= 1,
      hidden => Hidden =:= 1,
      system => System =:= 1}.
```

## Error Handling

```erlang
%% "Let it crash" — supervisors handle recovery
%% But validate at system boundaries:

%% Tagged tuples for expected results
case file:read_file(Path) of
    {ok, Content} -> process(Content);
    {error, enoent} -> create_default(Path);
    {error, Reason} -> {error, {file_read_failed, Reason}}
end.

%% try/catch for unexpected errors (rare)
try
    dangerous_operation()
catch
    error:badarg -> handle_badarg();
    throw:Reason -> handle_throw(Reason);
    exit:Reason -> handle_exit(Reason)
after
    cleanup()
end.
```

## Key Rules

1. **Let it crash.** Design for recovery, not prevention. Supervisors handle failures.
2. **Use OTP behaviors.** `gen_server`, `gen_statem`, `supervisor` — don't roll your own process loops.
3. **Processes are cheap.** Spawn thousands. Each Erlang process is ~300 bytes.
4. **Share nothing between processes.** All communication via message passing. No shared memory.
5. **Use binary pattern matching for protocols.** It's Erlang's most powerful feature for network code.
6. **Tag everything.** `{ok, Value}`, `{error, Reason}` — always use tagged tuples for return values.

---

*Sources: Learn You Some Erlang (Fred Hébert), Erlang/OTP documentation, Designing for Scalability with Erlang/OTP (Cesarini & Vinoski), Programming Erlang (Joe Armstrong)*
