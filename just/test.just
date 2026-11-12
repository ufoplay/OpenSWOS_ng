
# Runs all the tests
[group('test')]
test test_name="":
  cargo test --workspace {{test_name}}


# Runs the tests against your own copy of the PC game, e.g.: just test_real_assets ~/Games/SWOS
[group('test')]
test_real_assets game_dir="/home/ufo/Temp/Games/OpenSWOS/DataSource/" test_name="":
  OPENSWOS_GAME_DIR="{{game_dir}}" cargo test --workspace {{test_name}}


# Runs the tests against a folder of OpenSWOS saves (yours, never committed), e.g.: just test_saves ~/.swos/save
[group('test')]
test_saves saves_dir="/home/ufo/Temp/Games/OpenSWOS/config/.swos/save/" test_name="":
  OPENSWOS_SAVES_DIR="{{saves_dir}}" cargo test --workspace {{test_name}}
