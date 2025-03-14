#!/usr/bin/env -S ruby -w

CONFIG_FILE = "#{__dir__}/rclone.conf"

DEFAULT_ACCESS_KEY = "GUDEUNV77O1V6VNPJWT1"
DEFAULT_SECRET_KEY = "nQ05UXRX3pRvYJUiSk7km6iMtLCy8pfx4yghPZSP"

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
