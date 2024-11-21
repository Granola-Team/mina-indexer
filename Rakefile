# Rakefile

root_dir = pwd

if ENV["FLOX_ENV"].nil?
  sh "flox activate" do |ok, status|
    if !ok
      puts "Failed to activate Flox environment. Is Flox installed? https://flox.dev/docs/install-flox/"
      exit 1
    end
  end
end

# Make task list the default action
task :default do
  system "rake --tasks"
end

desc "Build"
task :build do
  chdir "rust"
  sh "cargo build"
  chdir root_dir
end

desc "Test"
task :test do
  chdir "rust"
  sh "cargo test --lib -- --test-threads=1"
  chdir root_dir
end

desc "Integration Tests"
task :it do
  chdir "rust"
  sh "cargo test --test regression_tests -- --test-threads=1"
  chdir root_dir
end

desc "Ingest blocks"
task ingest: [:build] do
  chdir "rust"
  sh "cargo run --bin ingest_blocks"
  chdir root_dir
end

desc "Ingest From Root"
task :ingest_from_root, [:height, :state_hash] => [:build] do |t, args|
  args.with_defaults(height: nil, state_hash: nil)

  # Construct the command
  cmd = "cargo run --bin ingest_blocks --"
  if args.height && args.state_hash
    cmd += " #{args.height} #{args.state_hash}"
  end

  # Run the command
  chdir "rust" do
    sh cmd
  end

  chdir root_dir
end


desc "Clean database"
task :clean_db do
  chdir "rust"
  sh "rm mina.db"
  chdir root_dir
end

desc "Clean up build artifacts"
task :clean do
  chdir "rust"
  sh "cargo clean"
  sh "rm mina.db"
  chdir root_dir
end

desc "Format code"
task :format do
  chdir "rust"
  sh "cargo-fmt --all"
  chdir root_dir
end

desc "Lint code"
task :lint do
  chdir "rust"
  sh "cargo machete --fix"
  sh "cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged -- -D warnings"
  chdir root_dir
  sh "standardrb --fix scripts"
end

desc "Check code formatting and run Clippy"
task :check do
  chdir "rust"
  sh "cargo machete"
  sh "cargo-fmt --all --check && cargo clippy --all-targets --all-features -- -D warnings"
  chdir root_dir
  sh "standardrb --no-fix scripts"
end

desc "Checks readiness of code"
task ready: [:lint, :format, :check, :test]
