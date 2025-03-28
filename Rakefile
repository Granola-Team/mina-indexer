require "fileutils"
require "open3"

TOP = __dir__

import "#{TOP}/ops/bin.rake"
import "#{TOP}/ops/stage-blocks.rake"

ENV["CARGO_HOME"] = "#{TOP}/.cargo"

# This required environment variable is used during the Rust compilation.
ENV["GIT_COMMIT_HASH"] ||= begin
  git_hash = `git rev-parse --short=8 HEAD 2>/dev/null`.strip
  git_hash.empty? ? "dev" : git_hash
end

IMAGE = "mina-indexer:#{ENV["GIT_COMMIT_HASH"]}"
BUILD_TYPE = "dev"
PROD_MODE = "nix"
REGRESSION_TEST = "./ops/regression-test.rb"
DEPLOY_TIER3 = "./ops/deploy-tier3.rb"
DEPLOY_PROD = "./ops/deploy-prod.rb"
UTILS = "./ops/utils.rb"

RUST_SRC_FILES = Dir.glob("rust/**/*").reject { |path| path.include?("rust/target/") }
CARGO_DEPS = [".cargo/config.toml"] + RUST_SRC_FILES
RUBY_SRC_FILES = Dir.glob("ops/**/*.rb") + Dir.glob("ops/**/*.rake") + ["Rakefile"]
NIX_FILES = %W[
  flake.nix
  ops/mina/mina_txn_hasher.nix
]
SHELL_SCRIPTS = %W[
  ops/ci/tier3
  ops/ci/prod
  ops/ci/tier1
  ops/ci/tier2
  tests/regression.bash
  tests/recovery.sh
  ops/download-snapshot.sh
]
# TODO:
#  ops/traverse-canonical-chain.sh
#  ops/correct-file-names.sh
#  ops/minaexplorer/download-staking-ledgers.sh
#  ops/upload-staking-ledgers.sh
#  ops/upload-snapshot.sh
#  ops/upload-mina-blocks.sh
#  ops/calculate-archive-ledgers.sh

def is_rustfmt_nightly
  stdout, _, _ = Open3.capture3("rustfmt --version | grep stable || echo \"true\"")
  stdout.strip == "true"
end

def nightly_if_required
  is_rustfmt_nightly ? "" : "+nightly"
end

def run(cmd, *args, dir: TOP)
  success = system(cmd, *args, chdir: dir)
  abort "Command failed: #{cmd} #{args.join(" ")}" unless success
  success
end

def cmd_output(cmd)
  output = ""
  IO.popen(cmd, err: [:child, :out]) do |io|
    while (line = io.gets)
      output += line
      print line
    end
  end
  unless $?.success?
    raise "Command '#{cmd}' failed with exit status #{$?.exitstatus}"
  end
  output
end

def cargo_output(subcmd)
  output = ""
  Dir.chdir("rust") do
    output = cmd_output("cargo #{subcmd}")
  end
  output
end

# Task aliases

task sd: "show:dev"
task sp: "show:prod"
task st: "show:test"

task cd: "clean:dev"
task cp: "clean:prod"
task ct: "clean:test"

task bt: :dev
task btc: "dev:continue"

task dlp: "deploy:local_prod_dev"

task tier1: "test:tier1"
task tier2: "test:tier2"

task :default do
  run "rake -T"
end

# Audit and linting tasks

desc "Perform Cargo audit"
task audit: [".build/cargo_audit"]

file ".build/cargo_audit": ["rust/Cargo.lock"] do |t|
  puts "--- Performing Cargo audit"
  FileUtils.mkdir_p(".build")
  cargo_output("--version")
  cargo_output("audit --version")
  audit_output = cargo_output("audit")
  File.write(t.name, audit_output)
end

desc "Lint Rust code with clippy"
task lint_rust: [:audit, ".build/cargo_clippy"]

file ".build/cargo_clippy": CARGO_DEPS do |t|
  puts "--- Linting Rust code with clippy"
  FileUtils.mkdir_p(".build")
  cargo_output("--version")
  cargo_output("clippy --version")
  clippy_output = cargo_output("clippy --all-targets --all-features \
    -- \
    -Dwarnings \
    -Dclippy::too_many_lines \
    -Dclippy::negative_feature_names \
    -Dclippy::redundant_feature_names \
    -Dclippy::wildcard_dependencies \
    -Dclippy::unused_self \
    -Dclippy::used_underscore_binding \
    -Dclippy::zero_sized_map_values")
  File.write(t.name, clippy_output)
  # Lints that demonstrably fail
  # -Dclippy::unused_async \
  # -Dclippy::multiple_crate_versions \
  # -Dclippy::cargo_common_metadata
  # -Dclippy::pedantic
  # -Dclippy::wildcard_imports
end

desc "Lint all Ruby code"
task lint_ruby: [".build/lint_ruby"]

file ".build/lint_ruby": RUBY_SRC_FILES do |t|
  puts "--- Linting Ruby code"
  FileUtils.mkdir_p(".build")
  run("ruby --version")
  run("standardrb --version")
  ruby_cw_out = cmd_output("ruby -cw #{RUBY_SRC_FILES.join(" ")}")
  standardrb_out = cmd_output("standardrb --no-fix #{RUBY_SRC_FILES.join(" ")}")
  File.write(t.name, [ruby_cw_out, standardrb_out].join("\n"))
end

desc "Lint shell scripts"
task lint_shell: [".build/lint_shell"]

file ".build/lint_shell": SHELL_SCRIPTS do |t|
  puts "--- Linting shell scripts"
  FileUtils.mkdir_p(".build")
  run("shellcheck --version")
  sc_out = cmd_output("shellcheck #{SHELL_SCRIPTS.join(" ")}")
  File.write(t.name, sc_out)
end

desc "Lint Nix code"
task lint_nix: [".build/lint_nix"]

file ".build/lint_nix": NIX_FILES do |t|
  puts "--- Linting Nix configs"
  FileUtils.mkdir_p(".build")
  run("alejandra --version")
  out = cmd_output("alejandra --check #{NIX_FILES}")
  File.write(t.name, out)
end

task cargo_machete: [".build/cargo_machete"]

file ".build/cargo_machete": CARGO_DEPS do |t|
  puts "--- Linting Cargo dependencies"
  FileUtils.mkdir_p(".build")
  cargo_output("--version")
  cargo_output("machete --version")
  machete_output = cargo_output("machete")
  File.write(t.name, machete_output)
end

desc "Lint all code"
task lint: [:lint_ruby, :lint_shell, :lint_nix, :cargo_machete, :lint_rust]

desc "Format all code"
task :format do
  # Format Rust code
  cargo_output("#{nightly_if_required} fmt --all")

  # Format Ruby code
  run("standardrb --fix #{RUBY_SRC_FILES.join(" ")}")

  # Format GraphQL in Hurl files - run from the toplevel directory
  run("ops/format-graphql-in-hurl-files.rb tests/hurl/")

  # Format shell scripts
  run("shfmt --version")
  run("shfmt --write ops/*.sh")
  run("shfmt --write tests/*.sh")
  run("shfmt --write tests/*.bash")

  # Format Nix files
  run("alejandra #{NIX_FILES.join(" ")}")
end

desc "Perform a fast verification of whether the source compiles"
task check: CARGO_DEPS do
  puts "--- Invoking 'cargo check'"
  cargo_output("check")
end

# Build tasks
namespace :build do
  desc "Perform a release build"
  task :prod do
    puts "--- Performing release build"
    run("nom build")
  end

  desc "Perform a dev build"
  task dev: CARGO_DEPS do
    cargo_output("build")
  end

  desc "Build OCI images"
  task :oci_image do
    puts "--- Building #{IMAGE}"
    run("docker --version")
    run("nom build .#dockerImage")
    run("docker load < ./result")
    run("docker run --rm -it #{IMAGE} mina-indexer server start --help")
    FileUtils.rm_f("result")
  end

  desc "Delete OCI image"
  task :delete_oci_image do
    puts "--- Deleting OCI image #{IMAGE}"
    run("docker image rm #{IMAGE}")
  end
end

# Show tasks
namespace :show do
  desc "Show mina-indexer PID(s)"
  task :pids do
    puts "Showing mina-indexer PID(s)"
    run("#{UTILS} pids show")
  end

  desc "Show the mina-indexer-dev directory"
  task :dev, [:which] do |_, args|
    which = args[:which] || "one"
    puts "Showing dev directory"
    run("#{UTILS} dev show #{which}")
  end

  desc "Show prod directories"
  task :prod do
    puts "Showing prod directory"
    run("#{UTILS} prod show")
  end

  desc "Show test directories"
  task :test do
    puts "Showing test directory"
    run("#{UTILS} test show")
  end
end

task clean: "clean:all"

# Clean tasks
namespace :clean do
  desc "Cargo clean & remove nix build"
  task :all do
    FileUtils.rm_rf(".build")
    FileUtils.rm_f("result")
    cargo_output("--version")
    cargo_output("clean")
    puts "Consider also 'git clean -xdfn'"
  end

  desc "Clean the mina-indexer-dev directory"
  task :dev, [:which] do |_, args|
    which = args[:which] || "one"
    puts "Cleaning dev directory"
    run("#{UTILS} dev clean #{which}")
  end

  desc "Clean mina-indexer-prod subdirectory"
  task :prod, [:which] do |_, args|
    which = args[:which] || "one"
    puts "Cleaning prod directory"
    run("#{UTILS} prod clean #{which}")
  end

  desc "Clean mina-indexer-test subdirectory"
  task :test do
    puts "Cleaning test directory"
    run("#{UTILS} test clean")
  end
end

# Dev tasks
namespace :download do
  desc "Download a specific mainnet PCB (based on height and state hash) from o1Labs' bucket"
  task :block, [:height, :state_hash, :dir] do |_, args|
    dir = args[:dir] || "."
    run("./ops/o1labs/download-mina-blocks.rb block #{args[:height]} #{args[:state_hash]} --dir #{dir}")
  end

  desc "Download all mainnet PCBs (at a specific height) from o1Labs' bucket"
  task :blocks, [:height, :dir] do |_, args|
    dir = args[:dir] || "."
    run("./ops/o1labs/download-mina-blocks.rb blocks #{args[:height]} --dir #{dir}")
  end
end

desc "Dev build and run regression tests"
task :dev, [:subtest] => "build:dev" do |_, args|
  subtest = args[:subtest] || ""
  run("#{REGRESSION_TEST} #{BUILD_TYPE} #{subtest}")
end

namespace :dev do
  desc "Debug build and continue regression tests from given test"
  task :continue, [:subtest] => "build:dev" do |_, args|
    subtest = args[:subtest] || ""
    run("#{REGRESSION_TEST} #{BUILD_TYPE} continue #{subtest}")
  end
end

# Test tasks
namespace :test do
  namespace :unit do
    desc "Run unit tests"
    task :tier1, [:test] => CARGO_DEPS do |_, args|
      test = args[:test] || ""
      puts "--- Invoking 'rspec ops/spec'"
      run("rspec ops/spec/*_spec.rb")
      puts "--- Performing tier 1 unit test(s)"
      cargo_output("nextest --version")
      cargo_output("nextest run #{test}")
    end

    desc "Run all feature unit tests (debug build)"
    task :tier2, [:test] do |_, args|
      test = args[:test] || ""
      puts "--- Performing all feature unit test(s)"
      cargo_output("nextest --version")
      cargo_output("nextest run --all-features #{test}")
    end
  end

  desc "Run regression test(s), either all of them or one named specific test"
  task :regression, [:subtest] do |_, args|
    subtest = args[:subtest] || ""
    puts "--- Performing regression tests #{subtest}"
    run("#{REGRESSION_TEST} #{BUILD_TYPE} #{subtest}")
  end

  # tier 2 regression tests
  tier2_regression_tests = ["load_v1", "load_v2", "best_chain_many_blocks", "all"]

  # Create regression test tasks dynamically
  namespace :regression do
    # Define a helper method to create regression test tasks
    def define_regression_test(name)
      desc "Run #{name} regression test"
      task name do
        test_name = (name == "all") ? nil : name
        Rake::Task["test:regression"].reenable
        Rake::Task["test:regression"].invoke(test_name)
      end
    end

    tier2_regression_tests.each do |test|
      define_regression_test(test)
    end
  end

  desc "Run the 1st tier of tests"
  task tier1: [:lint, "test:unit:tier1"] do
    puts "--- Performing tier 1 regression tests"
    run("#{REGRESSION_TEST} #{BUILD_TYPE} \
      ipc_is_available_immediately \
      clean_shutdown \
      clean_kill \
      account_balance_cli \
      best_chain_v1 \
      rest_accounts_summary \
      hurl_v1")
  end

  desc "Run the 2nd tier of tests"
  task tier2: ["test:unit:tier2"] do
    puts "--- Checking for tier-2 prereqs"
    run("jq --version")
    run("check-jsonschema --version")
    run("hurl --version")
    Rake::Task["build:dev"].invoke
    puts "--- Running tier-2 tests"
    tier2_regression_tests.map { |test|
      Rake::Task["test:regression:#{test}"].invoke
    }
  end

  namespace :tier3 do
    desc "Run the 3rd tier of tests with Nix-built binary"
    task :prod, [:blocks] => ["build:prod", "build:oci_image", "build:delete_oci_image"] do |_, args|
      blocks = args[:blocks] || "5000"
      puts "--- Performing tier3 regression tests with Nix-built binary"
      run("#{DEPLOY_TIER3} #{PROD_MODE} #{blocks}")
    end

    desc "Run the 3rd tier of tests with dev build & no unit tests"
    task :dev, [:blocks] => "build:dev" do |_, args|
      blocks = args[:blocks] || "5000"
      puts "--- Performing tier3 regression tests with dev-built binary"
      run("#{DEPLOY_TIER3} dev #{blocks}")
    end
  end
end

# Deploy tasks
namespace :deploy do
  desc "Run a server as if in production with the release-built binary"
  task :local_prod, [:blocks, :web_port] => "build:prod" do |_, args|
    blocks = args[:blocks] || "5000"
    web_port = args[:web_port] || ""
    puts "--- Deploying prod indexer"
    run("#{DEPLOY_PROD} #{PROD_MODE} #{blocks} #{web_port}")
  end

  desc "Run a server as if in production with the dev-built binary"
  task :local_prod_dev, [:blocks, :web_port] do |_, args|
    blocks = args[:blocks] || "5000"
    web_port = args[:web_port] || ""
    Rake::Task["test:tier3:dev"].invoke(blocks)
    puts "--- Deploying dev prod indexer"
    run("#{DEPLOY_PROD} dev #{blocks} #{web_port}")
  end
end

desc "Shutdown a running local test/dev/prod indexer"
task :shutdown, [:which] do |_, args|
  which = args[:which] || "dev"
  puts "Shutting down #{which} indexer"
  run("#{UTILS} #{which} shutdown")
  puts "Successfully shutdown. You may also want to do 'rake clean:#{which}'"
end
