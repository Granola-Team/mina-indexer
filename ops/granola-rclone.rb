#!/usr/bin/env -S ruby -w

CONFIG_FILE = "#{__dir__}/rclone.conf"

DEFAULT_ACCESS_KEY = "WLCHU2I1ZK1C6DCMEXOY"
DEFAULT_SECRET_KEY = "VTYAeOyxYvYI6iwEO8wRLwYemwiaAQ59Z7AtidEH"

access_key = ENV["LINODE_OBJ_ACCESS_KEY"] || DEFAULT_ACCESS_KEY
secret_key = ENV["LINODE_OBJ_SECRET_KEY"] || DEFAULT_SECRET_KEY

args = [
  "rclone",
  "--config", CONFIG_FILE,
  "--buffer-size=128Mi",
  "--log-level=INFO",
  "--s3-access-key-id=#{access_key}",
  "--s3-secret-access-key=#{secret_key}",
  *ARGV
]
warn "granola-rclone issuing: #{args}"
system(*args) || abort("rclone failed")
