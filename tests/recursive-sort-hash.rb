def sort_recursively(obj)
  case obj
  when Hash
    obj.keys.sort.each_with_object({}) do |key, sorted_hash|
      sorted_hash[key] = sort_recursively(obj[key])
    end
  when Array
    sorted_array = obj.sort
    sorted_array.map { |item| sort_recursively(item) }
  else
    obj
  end
end
