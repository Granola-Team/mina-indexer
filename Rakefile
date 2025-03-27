require "fileutils"
require "open3"

# Environment variables
RAKEFILE_PATH = File.expand_path(__FILE__)
ENV["TOPLEVEL"] = File.dirname(RAKEFILE_PATH)
ENV["CARGO_HOME"] = "#{ENV["TOPLEVEL"]}/.cargo"
RUST_DIR = File.join(ENV["TOPLEVEL"], "rust")
# Set GIT_COMMIT_HASH if not already set
ENV["GIT_COMMIT_HASH"] ||= `git rev-parse --short=8 HEAD 2>/dev/null`.strip.tap { |hash| break "dev" if hash.empty? }

IMAGE = "mina-indexer:#{ENV["GIT_COMMIT_HASH"]}"

# Constants
BUILD_TYPE = "dev"
PROD_MODE = "nix"
REGRESSION_TEST = "./ops/regression-test.rb"
DEPLOY_TIER3 = "./ops/deploy-tier3.rb"
DEPLOY_PROD = "./ops/deploy-prod.rb"
UTILS = "./ops/utils.rb"

# Helper methods
def is_rustfmt_nightly
  stdout, _, _ = Open3.capture3("rustfmt --version | grep stable || echo \"true\"")
  stdout.strip == "true"
end

def nightly_if_required
  is_rustfmt_nightly ? "" : "+nightly"
end

def run_in_rust_dir(cmd)
  if Dir.pwd == RUST_DIR
    run(cmd, dir: RUST_DIR)
  else
    Dir.chdir(RUST_DIR) { run(cmd, dir: RUST_DIR) }
  end
end

def run(cmd, *args, dir: ENV["TOPLEVEL"])
  success = system(cmd, *args, chdir: dir)
  abort "Command failed: #{cmd} #{args.join(" ")}" unless success
  success
end

def run_silent(cmd, *args) # standard:disable all
  _, status = Open3.capture2e(cmd, *args) # standard:disable all
  status.success?
end

# Include other rake files (necessary if running using `rake -f`)
Dir.glob(File.join(ENV["TOPLEVEL"], "ops", "*.rake")).each { |r| import r }

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

# for backwards compatibility
task "build:nix": "build:prod"
task "test:tier3:nix": "test:tier3:prod"

task default: ["list"]

desc "List available tasks"
task :list do
  run "rake -T"
end

# Prerequisite checks
namespace :prereqs do
  desc "Check for presence of tier 1 dependencies"
  task :tier1 do
    puts "--- Checking for tier-1 prereqs"
    run_in_rust_dir("cargo --version")
    run_in_rust_dir("cargo nextest --version")
    run_in_rust_dir("cargo audit --version")
    run_in_rust_dir("cargo machete --version")
    run("shellcheck --version")
    run("shfmt --version")
  end

  desc "Check for presence of tier 2 dependencies"
  task tier2: "prereqs:tier1" do
    puts "--- Checking for tier-2 prereqs"
    run("jq --version")
    run("check-jsonschema --version")
    run("hurl --version")
  end
end

# Audit and linting tasks

desc "Perform Cargo audit"
task :audit do
  puts "--- Performing Cargo audit"
  run_in_rust_dir("time cargo audit")
end

file ".build" do |t|
  FileUtils.mkdir_p(t.name)
end

desc "Lint Rust code with clippy"
task clippy: [".build/clippy"]

RUST_SRC_FILES = Dir.glob("rust/**")

file ".build/clippy": [".build"] + RUST_SRC_FILES do
  puts "--- Linting Rust code with clippy"
  run_in_rust_dir("cargo --version")
  run_in_rust_dir("cargo clippy --version")
  run_in_rust_dir("cargo clippy --all-targets --all-features \
    -- \
    -Dwarnings \
    -Dclippy::too_many_lines \
    -Dclippy::negative_feature_names \
    -Dclippy::redundant_feature_names \
    -Dclippy::wildcard_dependencies \
    -Dclippy::unused_self \
    -Dclippy::used_underscore_binding \
    -Dclippy::zero_sized_map_values \
    2>&1 | tee ../.build/clippy")
  # Lints that demonstrably fail
  # -Dclippy::unused_async \
  # -Dclippy::multiple_crate_versions \
  # -Dclippy::cargo_common_metadata
  # -Dclippy::pedantic
  # -Dclippy::wildcard_imports
end

RUBY_SRC_FILES = Dir.glob("ops/**/*.rb") + Dir.glob("ops/**/*.rake") + ["Rakefile"]

desc "Lint all Ruby code"
task lint_ruby: [".build/lint_ruby"]

file ".build/lint_ruby": [".build"] + RUBY_SRC_FILES do |t|
  puts "--- Linting Ruby code"
  run("ruby --version")
  run("standardrb --version")
  ruby_cw_out = run_command("ruby -cw #{RUBY_SRC_FILES.join(" ")}")
  standardrb_out = run_command("standardrb --no-fix #{RUBY_SRC_FILES.join(" ")}")
  File.write(t.name, [ruby_cw_out, standardrb_out].join("\n"))
end

desc "Lint shell scripts"
task lint_shell: [".build/lint_shell"]

SHELL_SCRIPTS = %W[
  ./ops/ci/tier3
  ./ops/ci/prod
  ./ops/ci/tier1
  ./ops/ci/tier2
  ./tests/regression.bash
  ./tests/recovery.sh
]
#  ./.hooks/pre-push
#  ./.hooks/pre-commit
#  ./ops/traverse-canonical-chain.sh
#  ./ops/correct-file-names.sh
#  ./ops/minaexplorer/download-staking-ledgers.sh
#  ./ops/download-snapshot.sh
#  ./ops/upload-staking-ledgers.sh
#  ./ops/upload-snapshot.sh
#  ./ops/upload-mina-blocks.sh
#  ./ops/calculate-archive-ledgers.sh

def run_command(cmd)
  output = `#{cmd}`

  unless $?.success?
    raise "Command '#{cmd}' failed with exit status #{$?.exitstatus} " \
          "and output:\n#{output}"
  end

  output
end

file ".build/lint_shell": [".build"] + SHELL_SCRIPTS do |t|
  puts "--- Linting regression scripts"
  sc_out = run_command("shellcheck #{SHELL_SCRIPTS.join(" ")}")
  File.write(t.name, sc_out)
end

desc "Lint all code"
task lint: [:clippy, :lint_ruby, :lint_shell] do
  puts "--- Linting Nix configs"
  run("alejandra --check flake.nix ops/mina/mina_txn_hasher.nix")
  puts "--- Linting Cargo dependencies"
  run_in_rust_dir("cargo machete")
end

desc "Format all code"
task :format do
  # Format Rust code
  run_in_rust_dir("cargo #{nightly_if_required} fmt --all")

  # Format Ruby code
  run("standardrb --fix \"ops/**/*.rb\" Rakefile rakelib/*.rake")

  # Format GraphQL in Hurl files - run from the toplevel directory
  run("ops/format-graphql-in-hurl-files.rb tests/hurl/")

  # Format shell scripts
  run_silent("shfmt --write ops/*.sh")
  run_silent("shfmt --write tests/*.sh")
  run_silent("shfmt --write tests/*.bash")

  # Format Nix files
  run_silent("alejandra flake.nix ops/mina/mina_txn_hasher.nix")
end

desc "Perform a fast verification of whether the source compiles"
task :check do
  puts "--- Invoking 'cargo check'"
  run_in_rust_dir("time cargo check")
end

# Build tasks
namespace :build do
  desc "Perform a release build"
  task :prod do
    puts "--- Performing release build"
    run("nom build")
  end

  desc "Perform a dev build"
  task :dev do
    run_in_rust_dir("cargo build")
  end

  desc "Build OCI images"
  task :oci_image do
    puts "--- Building #{IMAGE}"
    run("docker --version")
    run("time nom build .#dockerImage")
    run("time docker load < ./result")
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
    run_in_rust_dir("cargo --version")
    run_in_rust_dir("cargo clean")
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

desc "Debug build and run regression tests"
task :dev, [:subtest] => "build:dev" do |_, args|
  subtest = args[:subtest] || ""
  run("time #{REGRESSION_TEST} #{BUILD_TYPE} #{subtest}")
end

namespace :dev do
  desc "Debug build and continue regression tests from given test"
  task :continue, [:subtest] => "build:dev" do |_, args|
    subtest = args[:subtest] || ""
    run("time #{REGRESSION_TEST} #{BUILD_TYPE} continue #{subtest}")
  end
end

# Test tasks
namespace :test do
  namespace :unit do
    desc "Run unit tests"
    task :tier1, [:test] do |_, args|
      test = args[:test] || ""
      puts "--- Invoking 'rspec ops/spec'"
      run("rspec ops/spec/*_spec.rb")
      puts "--- Performing tier 1 unit test(s)"
      run_in_rust_dir("time cargo nextest run #{test}")
    end

    desc "Run all feature unit tests (debug build)"
    task :tier2, [:test] do |_, args|
      test = args[:test] || ""
      puts "--- Performing all feature unit test(s)"
      run_in_rust_dir("time cargo nextest run --all-features #{test}")
    end
  end

  desc "Run regression test(s), either all of them or one named specific test"
  task :regression, [:subtest] do |_, args|
    subtest = args[:subtest] || ""
    puts "--- Performing regression tests #{subtest}"
    run("time #{REGRESSION_TEST} #{BUILD_TYPE} #{subtest}")
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
  task tier1: ["prereqs:tier1", :lint, "test:unit:tier1"] do
    puts "--- Performing tier 1 regression tests"
    run("time #{REGRESSION_TEST} #{BUILD_TYPE} \
      ipc_is_available_immediately \
      clean_shutdown \
      clean_kill \
      block_copy \
      account_balance_cli \
      best_chain_v1 \
      rest_accounts_summary \
      reuse_databases \
      hurl_v1")
  end

  desc "Run the 2nd tier of tests"
  task tier2: ["prereqs:tier2", "test:unit:tier2", "build:dev"] +
    tier2_regression_tests.map { |test| "test:regression:#{test}" }

  namespace :tier3 do
    desc "Run the 3rd tier of tests with Nix-built binary"
    task :prod, [:blocks] => ["build:prod", "build:oci_image", "build:delete_oci_image"] do |_, args|
      blocks = args[:blocks] || "5000"
      puts "--- Performing tier3 regression tests with Nix-built binary"
      run("time #{DEPLOY_TIER3} #{PROD_MODE} #{blocks}")
    end

    desc "Run the 3rd tier of tests with dev build & no unit tests"
    task :dev, [:blocks] => "build:dev" do |_, args|
      blocks = args[:blocks] || "5000"
      puts "--- Performing tier3 regression tests with dev-built binary"
      run("time #{DEPLOY_TIER3} dev #{blocks}")
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
    run("time #{DEPLOY_PROD} #{PROD_MODE} #{blocks} #{web_port}")
  end

  desc "Run a server as if in production with the dev-built binary"
  task :local_prod_dev, [:blocks, :web_port] do |_, args|
    blocks = args[:blocks] || "5000"
    web_port = args[:web_port] || ""
    Rake::Task["test:tier3:dev"].invoke(blocks)
    puts "--- Deploying dev prod indexer"
    run("time #{DEPLOY_PROD} dev #{blocks} #{web_port}")
  end
end

desc "Shutdown a running local test/dev/prod indexer"
task :shutdown, [:which] do |_, args|
  which = args[:which] || "dev"
  puts "Shutting down #{which} indexer"
  run("#{UTILS} #{which} shutdown")
  puts "Successfully shutdown. You may also want to do 'rake clean:#{which}'"
end
