#!/usr/bin/env python3
"""
导出 SQLite 数据库到 SQL 文件
用于更新 public_init.sql 以同步最新的 RPC 配置

注意：此脚本导出所有表的结构，但只导出非敏感表的数据
"""

import sqlite3
import os
from datetime import datetime


def get_table_schema(cursor, table_name):
    """获取表的创建语句"""
    cursor.execute(
        f"SELECT sql FROM sqlite_master WHERE type='table' AND name='{table_name}'"
    )
    result = cursor.fetchone()
    return result[0] if result else None


def get_table_columns(cursor, table_name):
    """获取表的列名"""
    cursor.execute(f"PRAGMA table_info({table_name})")
    return [row[1] for row in cursor.fetchall()]


def escape_sql_value(value):
    """转义 SQL 值"""
    if value is None:
        return "NULL"
    elif isinstance(value, str):
        # 转义单引号
        escaped = value.replace("'", "''")
        return f"'{escaped}'"
    elif isinstance(value, int):
        return str(value)
    elif isinstance(value, float):
        return str(value)
    elif isinstance(value, bytes):
        # BLOB 数据转为十六进制
        return f"X'{value.hex()}'"
    else:
        return f"'{str(value)}'"


def export_table_data(cursor, table_name):
    """导出表数据为 INSERT 语句"""
    columns = get_table_columns(cursor, table_name)
    cursor.execute(f"SELECT * FROM {table_name}")
    rows = cursor.fetchall()

    if not rows:
        return []

    inserts = []
    for row in rows:
        values = [escape_sql_value(value) for value in row]
        insert_sql = f"INSERT OR IGNORE INTO {table_name} ({', '.join(columns)}) VALUES ({', '.join(values)});"
        inserts.append(insert_sql)

    return inserts


def export_database_to_sql(db_path, output_path):
    """导出整个数据库到 SQL 文件"""

    if not os.path.exists(db_path):
        print(f"错误: 数据库文件不存在: {db_path}")
        return False

    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    # 获取所有表（排除 sqlite_ 系统表）
    cursor.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
    )
    all_tables = [row[0] for row in cursor.fetchall()]

    # 定义包含敏感数据的表（只导出结构，不导出数据）
    # - wallets: 包含钱包私钥、助记词
    # - wallet_groups: 钱包分组信息
    # - app_config: 包含主密码验证器和加密密钥
    sensitive_tables = {"wallets", "wallet_groups", "app_config"}

    # 定义表的依赖顺序（被依赖的表在前）
    table_order = [
        "chains",  # 基础表
        "rpc_providers",  # 依赖 chains
        "tokens",  # 依赖 chains
        "notification_configs",
        "monitor_configs",
        "monitor_history",  # 依赖 monitor_configs
        "app_config",
    ]

    # 按照预定义顺序排序所有表
    ordered_tables = []

    # 先添加预定义顺序中的表
    for t in table_order:
        if t in all_tables:
            ordered_tables.append(t)

    # 添加其他表（包括敏感表，但只导出结构）
    for t in all_tables:
        if t not in ordered_tables:
            ordered_tables.append(t)

    sql_content = []
    sql_content.append("-- Wallet Manager 数据库初始化脚本")
    sql_content.append(f"-- 生成时间: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    sql_content.append("-- 此文件包含所有表结构，但只包含非敏感配置数据")
    sql_content.append("-- 敏感表（wallets, wallet_groups）只包含结构，不包含数据")
    sql_content.append("")

    # 第一步：删除已存在的表（清理旧数据）
    sql_content.append("-- ============================================")
    sql_content.append("-- 第一步：删除已存在的表（清理旧数据）")
    sql_content.append("-- ============================================")
    # 按依赖关系的逆序删除表（先删除有外键依赖的表）
    drop_order = list(reversed(ordered_tables))
    for table_name in drop_order:
        sql_content.append(f"DROP TABLE IF EXISTS {table_name};")
    sql_content.append("")

    # 第二步：创建所有表结构
    sql_content.append("-- ============================================")
    sql_content.append("-- 第二步：创建所有表结构")
    sql_content.append("-- ============================================")
    sql_content.append("")

    for table_name in ordered_tables:
        print(f"正在导出表结构: {table_name}")

        # 获取表结构
        schema = get_table_schema(cursor, table_name)
        if schema:
            sql_content.append(f"-- 创建{table_name}表")
            sql_content.append(f"{schema};")
            sql_content.append("")

    # 第三步：插入非敏感表数据
    sql_content.append("")
    sql_content.append("-- ============================================")
    sql_content.append("-- 第三步：插入非敏感表数据")
    sql_content.append("-- ============================================")
    sql_content.append("")

    for table_name in ordered_tables:
        if table_name in sensitive_tables:
            print(f"跳过敏感数据: {table_name}")
            sql_content.append(f"-- 表 {table_name} 包含敏感数据，跳过数据导出")
            sql_content.append("")
            continue

        print(f"正在导出表数据: {table_name}")

        # 添加表数据
        inserts = export_table_data(cursor, table_name)
        if inserts:
            sql_content.append(f"-- 插入{table_name}表数据")
            sql_content.extend(inserts)
            sql_content.append("")

    # 第四步：创建索引
    sql_content.append("")
    sql_content.append("-- ============================================")
    sql_content.append("-- 第四步：创建索引")
    sql_content.append("-- ============================================")
    sql_content.append("")

    cursor.execute(
        "SELECT name, sql FROM sqlite_master WHERE type='index' AND sql IS NOT NULL AND name NOT LIKE 'sqlite_%'"
    )
    indexes = cursor.fetchall()

    if indexes:
        # 先删除所有索引
        for index_name, _ in indexes:
            sql_content.append(f"DROP INDEX IF EXISTS {index_name};")
        sql_content.append("")
        # 再创建所有索引
        for _, index_sql in indexes:
            sql_content.append(f"{index_sql};")

    conn.close()

    # 写入文件
    with open(output_path, "w", encoding="utf-8") as f:
        f.write("\n".join(sql_content))

    print(f"\n导出完成!")
    print(f"输出文件: {output_path}")
    print(f"文件大小: {os.path.getsize(output_path)} 字节")
    print(f"导出表数: {len(ordered_tables)}")
    print(f"敏感表（仅结构，不导出数据）: {', '.join(sensitive_tables)}")

    return True


if __name__ == "__main__":
    # 数据库路径
    db_path = os.path.join(
        os.path.dirname(__file__), "..", "src-tauri", "data", "wallets_tool.db"
    )
    output_path = os.path.join(
        os.path.dirname(__file__), "..", "src-tauri", "data", "public_init.sql"
    )

    # 转换为绝对路径
    db_path = os.path.abspath(db_path)
    output_path = os.path.abspath(output_path)

    print(f"数据库路径: {db_path}")
    print(f"输出路径: {output_path}")
    print("-" * 50)

    export_database_to_sql(db_path, output_path)
