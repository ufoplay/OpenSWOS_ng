import "./just/build.just"
import "./just/code_check.just"
import "./just/test.just"
import "./just/upstream.just"
import "./just/worlds.just"

export RUST_BACKTRACE := "full"


# Lists all the available commands
default:
  @just --list
