#! /bin/bash

function urlencode() {
  which "curl" >/dev/null 2>&1
  if [ ! $? -eq 0 ]; then
    echo -E "$1"
    return
  fi
  encode_str=$(echo -E "$1" | sed "s/%/%%/g")
  printf -- "$encode_str" | curl -Gso /dev/null -w %{url_effective} --data-urlencode @- "" | cut -c 3-
}

# 指定CSV文件所在的目录和生成图片的目录
csv_dir="data/dual/AMPK-JAK-LINK"

# 定义函数，用于处理单个CSV文件
process_csv() {
  # 获取文件名（不含扩展名）
  image_dir="data/SMILES"

  mkdir -p $image_dir
  # 读取CSV文件的第一列smiles
  for item in $(cut -d ',' -f 1 "$1" | tail -n +2); do
    file_name=$(urlencode "$item")
    # if [ ! -f "$image_dir/$file_name.png" ]; then
    obabel -:"$item" -O "$image_dir/$file_name.svg" -osvg -xb none
    # fi
  done

  # 调用obabel生成图片
}

# 导出函数以便parallel调用
export -f urlencode
export -f process_csv

# 使用parallel并行处理所有CSV文件
find "$csv_dir" -name '*.csv' -print0 | parallel -0 -j+24 "process_csv {}"

echo "处理完成！"
