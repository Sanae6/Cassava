def main [stage: string] {
  mkdir romfs
  cargo run --example sarc read $"../romfs/StageData/($stage).szs" $"romfs/($stage)"
}
