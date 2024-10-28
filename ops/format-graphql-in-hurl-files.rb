#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

require "json"
require "tempfile"
require "find"

class GraphQLFormatter
  class << self
    def process_directory(dir_path)
      Find.find(dir_path) do |path|
        next unless path.end_with?(".hurl")
        process_file(path)
      end
    end

    def process_file(file_path)
      content = File.read(file_path)
      processed_content = process_graphql_content(content)
      File.write(file_path, processed_content)
    end

    def process_graphql_content(content)
      content.gsub(/```graphql\n(.*?)\n```/m) do |match|
        query_section = $1.strip

        needs_processing = false

        if query_section.include?("variables {")
          query_part, variables_part = query_section.split(/\n\nvariables\s*{/)
          variables_json = variables_part.strip.sub(/}\s*$/, "")
          variables = JSON.parse("{#{variables_json}}")

          query_body = extract_query_body(query_part)
          query_section = inline_variables(query_body, variables)
          needs_processing = true
        end

        if needs_processing || needs_processing?(query_section)
          cleaned_query = remove_query_definition(query_section)
          unquoted_query = remove_sort_by_quotes(cleaned_query)
          query_section = reorder_arguments(unquoted_query)
        end

        formatted_query = format_with_biome(query_section)
        "```graphql\n#{formatted_query}\n```"
      end
    end

    private

    def needs_processing?(query)
      # Check if the query has variables that need to be inlined
      return true if query.include?("variables {")

      # Check if the query has a query definition
      return true if query.match?(/^query\s+\w+.*?{/m)

      # Check if sortBy has quotes
      return true if query.match?(/sortBy:\s*"[A-Z_]+"/)

      # Check if arguments need reordering
      if query =~ /\w+\(([\s\S]*?)\)/
        args_str = $1
        args = parse_arguments(args_str)
        return false if args.empty?

        ordered_keys = ["limit", "sortBy", "query"]
        current_order = args.keys

        # Check if the current order matches our desired order
        current_ordered_keys = ordered_keys & current_order
        return true if current_ordered_keys != current_order.select { |k| ordered_keys.include?(k) }
      end

      false
    end

    def remove_sort_by_quotes(query)
      query.gsub(/sortBy:\s*"([A-Z_]+)"/, 'sortBy: \1')
    end

    def remove_query_definition(query)
      if query.match?(/^query\s+\w+.*?{/m)
        query.sub(/^query\s+\w+.*?{/m, "{")
      else
        query
      end
    end

    def extract_query_body(query_part)
      if query_part.include?("query ")
        match = query_part.match(/{\s*(.*)\s*}\s*$/m)
        if match
          content = match[1].strip
          content.split("\n").map(&:strip).join("\n")
        else
          query_part
        end
      else
        query_part
      end
    end

    def inline_variables(query_body, variables)
      variables.each do |key, value|
        query_body.gsub!("$#{key}", format_value(value, key))
      end
      "{\n#{query_body}\n}"
    end

    def format_value(value, key = nil)
      case value
      when Hash
        format_graphql_object(value)
      when String
        if key == "sort_by" || value.match?(/^[A-Z_]+$/)
          value
        else
          "\"#{value}\""
        end
      else
        value.to_s
      end
    end

    def format_graphql_object(obj)
      pairs = obj.map do |k, v|
        formatted_value = format_value(v, k)
        "#{k}: #{formatted_value}"
      end
      "{#{pairs.join(", ")}}"
    end

    def reorder_arguments(query)
      query.gsub(/(\w+)\(([\s\S]*?)\)(\s*{)/) do |match|
        field_name = $1
        args_str = $2
        trailing_brace = $3

        # Parse arguments more carefully to handle nested objects
        args = parse_arguments(args_str)
        ordered_args = {}

        args.each do |key, value|
          ordered_args[key] = value
        end

        ordered_keys = ["limit", "sortBy", "query"]
        other_keys = ordered_args.keys - ordered_keys
        final_keys = (ordered_keys & ordered_args.keys) + other_keys

        formatted_args = final_keys.map do |key|
          "#{key}: #{ordered_args[key]}"
        end.join(", ")

        "#{field_name}(#{formatted_args})#{trailing_brace}"
      end
    end

    def parse_arguments(args_str)
      result = {}
      current_key = nil
      buffer = ""
      nesting_level = 0

      args_str.scan(/([^,{}:]+:|[{},:]|\s+|[^,{}:\s]+)/) do |token|
        token = token[0]
        case token
        when /(.+):/
          if nesting_level == 0
            current_key = $1.strip
            buffer = ""
          else
            buffer += token
          end
        when "{"
          nesting_level += 1
          buffer += token
        when "}"
          nesting_level -= 1
          buffer += token
          if nesting_level == 0 && current_key
            result[current_key] = buffer.strip
            current_key = nil
            buffer = ""
          end
        when ","
          if nesting_level == 0 && current_key
            result[current_key] = buffer.strip
            current_key = nil
            buffer = ""
          else
            buffer += token
          end
        else
          buffer += token unless token.strip.empty?
        end
      end

      # Handle the last argument if any
      if current_key && !buffer.empty?
        result[current_key] = buffer.strip
      end

      result
    end

    def format_with_biome(query)
      Tempfile.create(["query", ".graphql"]) do |f|
        f.write(query)
        f.flush
        `biome format --indent-style space --indent-width 2 --write #{f.path}`
        File.read(f.path).strip
      end
    rescue => e
      puts "Warning: Biome formatting failed (#{e.message}). Using unformatted query."
      query
    end
  end
end

if __FILE__ == $PROGRAM_NAME
  if ARGV.empty?
    puts "Usage: #{$0} <directory_or_file>"
    exit 1
  end

  path = ARGV[0]
  if File.directory?(path)
    GraphQLFormatter.process_directory(path)
  else
    GraphQLFormatter.process_file(path)
  end
end
