# persistent task

Sometimes you may want to have a continuously-executing thread that can do whatever you need.
This is not currently possible in the wasmtime component executor due to complexities around
how crashing threads should be handled.

Trying to build and run this module will produce an error.