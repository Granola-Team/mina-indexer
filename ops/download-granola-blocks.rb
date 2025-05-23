#!/usr/bin/env -S ruby -w

# Downloads historical Mina "stripped" precomputed blocks from the Granola
# storage bucket using `rclone`.

START_BLOCK = ARGV[0]
END_BLOCK = ARGV[1]
DEST = ARGV[2]

require "fileutils"

FileUtils.mkdir_p(DEST)

# Fetch the blocks list, if not already present.
#
blocks_list = "#{DEST}/../blocks.list"

unless File.exist?(blocks_list)
  warn "#{blocks_list} does not exist. Fetching..."

  cmd = "#{__dir__}/granola-rclone.rb lsf linode-granola:blocks.minasearch.com"
  warn "download-granola-blocks issuing: #{cmd}"

  contents = `#{cmd}`
  abort("Failure: #{cmd}") unless $?.success?

  new_list = contents.lines(chomp: true).sort! do |a, b|
    a_split = a.split("-")
    b_split = b.split("-")

    a_num = a_split[1].to_i
    b_num = b_split[1].to_i

    if a_num < b_num
      -1
    elsif a_num > b_num
      1
    else
      a_split[2] <=> b_split[2]
    end
  end

  File.write(blocks_list, new_list.join("\n"))
end

# Build the list of files that must be fetched.
#
list = File.readlines(blocks_list, chomp: true)
list = list.drop_while { |f| f.split("-")[1].to_i < START_BLOCK.to_i }
list = list.take_while { |f| f.split("-")[1].to_i <= END_BLOCK.to_i }

# Check to see if they're already present, building the list to fetch.
#
fetch = list.drop_while do |f|
  File.exist?("#{DEST}/#{f}")
end

if fetch.empty?
  warn "All files already present in #{DEST}."
else
  File.write("files-to-fetch.list", fetch.join("\n"))
  args = [
    "#{__dir__}/granola-rclone.rb",
    "sync",
    "linode-granola:blocks.minasearch.com",
    DEST,
    "--files-from-raw", "files-to-fetch.list"
  ]
  puts "Invoking: #{args}"
  system(*args) || abort("Download of Mina blocks failed.")

  File.delete("files-to-fetch.list")
end
