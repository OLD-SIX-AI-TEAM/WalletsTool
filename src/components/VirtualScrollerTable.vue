<template>
  <div ref="tableContainerRef" class="virtual-scroller-table" :style="{ height: height, width: '100%' }">
    <!-- 表头 -->
    <div ref="headerRef" class="table-header">
      <div class="header-row" :style="{ paddingRight: scrollbarPadding }">
        <!-- 选择列 -->
        <div v-if="rowSelection" class="header-cell checkbox-cell">
          <input
            type="checkbox"
            :checked="isAllSelected"
            :indeterminate="isIndeterminate"
            @change="toggleSelectAll"
            class="checkbox"
          />
        </div>
        <!-- 普通列 -->
        <div
          v-for="column in sortedColumns"
          :key="column.dataIndex || column.slotName"
          :class="[
            'header-cell',
            {
              'sticky-right': isFixedRightColumn(column),
              'sticky-right-first': isFirstFixedRightColumn(column),
              'optional-cell': isOptionalColumn(column),
            },
          ]"
          :style="{
            ...getHeaderColumnStyle(column),
            ...getStickyRightStyle(column),
            textAlign: column.align || 'left',
          }"
        >
          {{ column.title }}
        </div>
      </div>
    </div>

    <!-- 虚拟滚动内容 -->
    <div
      class="table-body"
      @wheel="handleWheel"
    >
      <!-- 空数据提示 -->
      <div v-if="showEmptyData" class="empty-data">
        <template v-if="pageType === 'balance'">
          <div class="empty-icon">
            <Icon icon="icon-park-outline:wallet" style="width: 64px; height: 64px;"/>
          </div>
          <div class="empty-text">还没有钱包数据</div>
          <div class="empty-text-second">请先导入钱包开始查询余额</div>
          <div>
            <a-button
              type="primary"
              style="margin-top: 12px"
              @click="$emit('open-manual-import')"
            >
              <icon icon="mdi:upload" :size="16" style="margin-right: 4px" />
              手动录入钱包
            </a-button>
            <a-button
              type="primary"
              style="margin-top: 12px;margin-left: 20px"
              status="success"
              @click="$emit('open-file-upload')"
            >
              <icon icon="mdi:upload" :size="16" style="margin-right: 4px" />
              上传文件导入
            </a-button>
            <a-button
              v-if="showSystemImport"
              type="primary"
              style="margin-top: 12px;margin-left: 20px"
              status="warning"
              @click="$emit('open-system-import')"
            >
              <icon icon="mdi:database-import" :size="16" style="margin-right: 4px" />
              从系统导入
            </a-button>
          </div>
        </template>
        <template v-else-if="pageType === 'monitor'">
          <div class="empty-icon">
            <Icon icon="icon-park-outline:wallet" style="width: 64px; height: 64px;"/>
          </div>
          <div class="empty-text">还没有监控数据</div>
        </template>
        <template v-else-if="pageType === 'wallet_manager'">
          <div class="empty-icon">
            <Icon icon="icon-park-outline:wallet" style="width: 64px; height: 64px;"/>
          </div>
          <div class="empty-text">还没有钱包数据</div>
          <div class="empty-text-second">请先添加钱包或批量导入</div>
        </template>
        <template v-else-if="pageType === 'system_import'">
          <div class="empty-icon">
            <Icon icon="icon-park-outline:wallet" style="width: 64px; height: 64px;"/>
          </div>
          <div class="empty-text">暂无可导入的钱包</div>
          <div class="empty-text-second">请切换分组或调整筛选条件</div>
        </template>
        <template v-else-if="pageType === 'transfer'">
          <div class="empty-icon">
            <Icon icon="icon-park-outline:wallet" :style="{ width: '64px', height: '64px' }" />
          </div>
          <div class="empty-text">还没有转账数据</div>
          <div class="empty-text-second">请先录入钱包或者上传文件开始批量转账</div>
          <div>
            <a-button
              type="primary"
              style="margin-top: 12px"
              @click="handleEmptyAction('manual')"
            >
              <icon icon="mdi:upload" :size="16" style="margin-right: 4px" />
              手动录入（慎用）
            </a-button>
            <a-button
              type="primary"
              style="margin-top: 12px;margin-left: 20px"
              status="success"
              @click="handleEmptyAction('upload')"
            >
              <icon icon="mdi:upload" :size="16" style="margin-right: 4px" />
              上传文件导入（推荐）
            </a-button>
            <a-button
              v-if="showSystemImport"
              type="primary"
              style="margin-top: 12px;margin-left: 20px"
              status="warning"
              @click="$emit('open-system-import')"
            >
              <icon icon="mdi:database-import" :size="16" style="margin-right: 4px" />
              从系统导入
            </a-button>
          </div>
          <div style="margin-top: 15px; display: flex; align-items: center; justify-content: center;">
            <Icon
              icon="mdi:info"
              :size="12"
              style="margin-right: 4px;"/>
            <span class="empty-text-second">
              支持CSV、XLSX格式文件，
              <a
                href="#"
                @click.prevent="handleEmptyAction('template')"
                style="color: #a2beff;"
              >
                下载
              </a>
              导入模板
            </span>
          </div>
        </template>
        <template v-else>
          <div class="empty-icon">
            <Icon icon="icon-park-outline:wallet" style="width: 64px; height: 64px;"/>
          </div>
          <div class="empty-text">暂无数据</div>
        </template>
      </div>

      <VirtualScroller
        v-else
        ref="scrollerRef"
        :items="data"
        :itemSize="35"
        class="virtual-scroller"
        @scroll="handleScroll"
      >
        <template #item="{ item, options }">
          <div
            class="table-row"
            :class="{
              selected: isRowSelected(item),
              'row-hovered': isRowHovered(item),
              clickable: true,
              'zebra-stripe': getItemIndex(item) % 2 === 1,
            }"
            v-memo="getMemoDeps(item)"
            @click="handleRowClick(item, getItemIndex(item))"
          >
            <!-- 选择列 -->
            <div v-if="rowSelection" class="table-cell checkbox-cell">
              <input
                type="checkbox"
                :checked="isRowSelected(item)"
                @change="toggleRowSelection(item)"
                @click.stop
                class="checkbox"
              />
            </div>
            <!-- 普通列 -->
            <div
              v-for="column in sortedColumns"
              :key="column.dataIndex || column.slotName"
              :class="[
                'table-cell',
                {
                  'copyable-cell': isCopyableColumn(column),
                  'sticky-right': isFixedRightColumn(column),
                  'sticky-right-first': isFirstFixedRightColumn(column),
                  'optional-cell': isOptionalColumn(column),
                },
              ]"
              :style="{
                ...getContentColumnStyle(column),
                ...getStickyRightStyle(column),
                textAlign: column.align || 'left',
              }"
              :title="getTooltipText(column, item)"
              @dblclick="handleCellDoubleClick($event, column, item)"
            >
              <!-- 可复制列使用Tooltip包装 -->
              <Tooltip
                v-if="isCopyableColumn(column)"
                content="双击可复制"
                position="top"
                :mouseEnterDelay="0.3"
                :mouseLeaveDelay="0.1"
              >
                <!-- 插槽内容 -->
                <template v-if="column.slotName">
                  <!-- 自动处理序号列 -->
                  <span v-if="column.slotName === 'index'">
                    {{ getItemIndex(item) + 1 }}
                  </span>
                  <!-- 其他插槽内容 -->
                  <slot
                    v-else
                    :name="column.slotName"
                    :record="item"
                    :rowIndex="getItemIndex(item)"
                  >
                    {{ item[column.dataIndex] }}
                  </slot>
                </template>
                <!-- 普通内容 -->
                <span v-else :class="{ ellipsis: column.ellipsis }">
                  {{ getDisplayText(column, item) }}
                </span>
              </Tooltip>
              <!-- 非可复制列正常显示 -->
              <template v-else>
                <!-- 插槽内容 -->
                <template v-if="column.slotName">
                  <!-- 自动处理序号列 -->
                  <span v-if="column.slotName === 'index'">
                    {{ getItemIndex(item) + 1 }}
                  </span>
                  <!-- 其他插槽内容 -->
                  <slot
                    v-else
                    :name="column.slotName"
                    :record="item"
                    :rowIndex="getItemIndex(item)"
                  >
                    {{ item[column.dataIndex] }}
                  </slot>
                </template>
                <!-- 普通内容 -->
                <span v-else :class="{ ellipsis: column.ellipsis }">
                  {{ getDisplayText(column, item) }}
                </span>
              </template>
            </div>
          </div>
        </template>
      </VirtualScroller>
    </div>

    <!-- Loading 遮罩 -->
    <div v-if="loading" class="loading-overlay">
      <div class="loading-spinner"></div>
      <div class="loading-text">加载中...</div>
    </div>
  </div>
</template>

<script setup>
import { computed, ref, shallowRef, watch, onMounted, onUnmounted, h, nextTick } from "vue";
import VirtualScroller from "primevue/virtualscroller";
import { Icon } from "@iconify/vue";
import { Message, Tooltip, Modal } from "@arco-design/web-vue";

// Props
const props = defineProps({
  columns: {
    type: Array,
    required: true,
  },
  data: {
    type: Array,
    default: () => [],
  },
  rowKey: {
    type: String,
    default: "id",
  },
  rowSelection: {
    type: Object,
    default: null,
  },
  selectedKeys: {
    type: Array,
    default: () => [],
  },
  loading: {
    type: Boolean,
    default: false,
  },
  height: {
    type: String,
    default: "100%",
  },
  hoverKeys: {
    type: Array,
    default: () => [],
  },
  pageType: {
    type: String,
    default: "transfer", // transfer | balance | monitor
  },
  emptyData: {
    type: Boolean,
    default: null, // null 表示自动根据 data.length 判断
  },
  showSystemImport: {
    type: Boolean,
    default: true, // 是否显示"从系统导入"按钮
  },
});

// Emits
const emit = defineEmits(["row-click", "update:selectedKeys", "open-manual-import", "open-file-upload", "open-system-import", "download-template"]);

// 私钥脱敏工具函数
const maskPrivateKey = (value) => {
  if (!value || typeof value !== 'string') return '';
  if (value.length <= 25) return '*'.repeat(value.length);
  return value.substring(0, 25) + '...';
};

const getPrivateKeyWithoutSuffix = (value) => {
  if (!value || typeof value !== 'string') return '';
  if (value.length <= 6) return value;
  return value.substring(0, value.length - 6);
};

const getPrivateKeySuffix = (value) => {
  if (!value || typeof value !== 'string') return '';
  if (value.length <= 6) return value;
  return value.substring(value.length - 6);
};

// 处理空数据页面按钮点击
const handleEmptyAction = (type) => {
  switch (type) {
    case 'manual':
      emit('open-manual-import');
      break;
    case 'upload':
      emit('open-file-upload');
      break;
    case 'template':
      emit('download-template');
      break;
  }
};

// 计算属性
const isAllSelected = computed(() => {
  if (!props.rowSelection || props.data.length === 0) return false;
  return props.data.every((item) => props.selectedKeys.includes(getRowKey(item)));
});

const isIndeterminate = computed(() => {
  if (!props.rowSelection || props.data.length === 0) return false;
  const selectedCount = props.data.filter((item) =>
    props.selectedKeys.includes(getRowKey(item))
  ).length;
  return selectedCount > 0 && selectedCount < props.data.length;
});

// 缓存列排序结果，避免重复计算
const sortedColumns = shallowRef([]);

// 监听 columns 变化时更新 sortedColumns
watch(() => props.columns, (newCols) => {
  if (!newCols || newCols.length === 0) {
    sortedColumns.value = [];
    return;
  }
  const regular = newCols.filter((col) => col.slotName !== "optional");
  const optional = newCols.filter((col) => col.slotName === "optional");
  sortedColumns.value = [...regular, ...optional];
}, { immediate: true });

// 缓存行索引映射，O(1) 复杂度获取索引
const itemIndexMap = computed(() => {
  const map = new Map();
  const data = props.data;
  for (let i = 0; i < data.length; i++) {
    map.set(getRowKey(data[i]), i);
  }
  return map;
});

// 方法
const getRowKey = (item) => {
  return item[props.rowKey];
};

const getItemIndex = (item) => {
  return itemIndexMap.value.get(getRowKey(item)) ?? -1;
};

// 判断是否显示空数据提示
const showEmptyData = computed(() => {
  if (props.emptyData !== null) {
    return props.emptyData;
  }
  return !props.loading && props.data.length === 0;
});

// 滚动条相关状态
const scrollerRef = ref(null);
const tableContainerRef = ref(null);
const headerRef = ref(null);
const scrollbarWidth = ref(0);
const hasScrollbar = ref(false);
const scrollbarPadding = computed(() => {
  return hasScrollbar.value ? scrollbarWidth.value + 'px' : '0';
});
let resizeObserver = null;

// 处理横向滚动，同步表头位置
const handleScroll = (event) => {
  if (headerRef.value && event.target) {
    headerRef.value.scrollLeft = event.target.scrollLeft;
  }
};

const checkScrollbar = () => {
  if (scrollerRef.value && scrollerRef.value.$el) {
    const el = scrollerRef.value.$el;
    // 检查是否有垂直滚动条
    hasScrollbar.value = el.scrollHeight > el.clientHeight;
  } else {
    hasScrollbar.value = false;
  }
};

onMounted(() => {
  // 计算滚动条宽度
  const outer = document.createElement('div');
  outer.style.visibility = 'hidden';
  outer.style.overflow = 'scroll';
  outer.style.msOverflowStyle = 'scrollbar';
  document.body.appendChild(outer);
  const inner = document.createElement('div');
  outer.appendChild(inner);
  scrollbarWidth.value = (outer.offsetWidth - inner.offsetWidth);
  outer.parentNode.removeChild(outer);

  // 监听窗口大小变化
  window.addEventListener('resize', () => {
    checkScrollbar();
    updateScrollerHeight();
  });
  
  // 监听容器大小变化
  if (tableContainerRef.value) {
    resizeObserver = new ResizeObserver(() => {
      checkScrollbar();
      updateScrollerHeight();
    });
    resizeObserver.observe(tableContainerRef.value);
  }
  
  // 初始检查
  nextTick(() => {
    checkScrollbar();
    updateScrollerHeight();
  });
});

onUnmounted(() => {
  window.removeEventListener('resize', checkScrollbar);
  if (resizeObserver) {
    resizeObserver.disconnect();
  }
});

// 更新虚拟滚动器高度
const updateScrollerHeight = () => {
  if (!scrollerRef.value || !scrollerRef.value.$el) return;
  const scrollerEl = scrollerRef.value.$el;
  const containerHeight = tableContainerRef.value?.clientHeight || 0;
  const bottomReserved = props.pageType === 'system_import' ? -2 : 30;
  // 减去表头高度(40)、边框误差(2)和底部状态栏预留空间
  const scrollerHeight = containerHeight - 40 - 2 - bottomReserved; 
  if (scrollerHeight > 0) {
    scrollerEl.style.height = `${scrollerHeight}px`;
  }
};

watch(() => props.data, () => {
  nextTick(() => {
    checkScrollbar();
    updateScrollerHeight();
  });
}, { deep: true, immediate: true });

// 计算表头列宽度（保持原始设置不变）
const getColumnKey = (column) => {
  return column.dataIndex || column.slotName || column.title;
};

const isOptionalColumn = (column) => {
  return column?.slotName === "optional";
};

const isFixedRightColumn = (column) => {
  return column?.fixed === "right" || isOptionalColumn(column);
};

const isFirstFixedRightColumn = (column) => {
  if (!isFixedRightColumn(column)) return false;
  const cols = sortedColumns.value || [];
  for (let i = 0; i < cols.length; i++) {
    if (isFixedRightColumn(cols[i])) {
      return getColumnKey(cols[i]) === getColumnKey(column);
    }
  }
  return false;
};

const getColumnWidth = (column) => {
  const width = Number(column?.width);
  if (Number.isFinite(width) && width > 0) return width;
  if (isFixedRightColumn(column)) return 120;
  return null;
};

const fixedRightOffsetMap = computed(() => {
  const cols = sortedColumns.value || [];
  const map = new Map();
  let offset = 0;
  for (let i = cols.length - 1; i >= 0; i--) {
    const col = cols[i];
    if (!isFixedRightColumn(col)) continue;
    map.set(getColumnKey(col), offset);
    offset += getColumnWidth(col) ?? 0;
  }
  return map;
});

const getStickyRightStyle = (column) => {
  if (!isFixedRightColumn(column)) return {};
  const offset = fixedRightOffsetMap.value.get(getColumnKey(column)) ?? 0;
  return { "--sticky-right-offset": `${offset}px` };
};

const fillColumnKey = computed(() => {
  const cols = sortedColumns.value || [];
  const firstRightIndex = cols.findIndex(isFixedRightColumn);
  if (firstRightIndex <= 0) return null;
  const hasFlexibleBeforeRight = cols.slice(0, firstRightIndex).some((col) => {
    const width = Number(col?.width);
    return !isFixedRightColumn(col) && !(Number.isFinite(width) && width > 0);
  });
  if (hasFlexibleBeforeRight) return null;
  const candidate = cols[firstRightIndex - 1];
  if (!candidate || isFixedRightColumn(candidate)) return null;
  return getColumnKey(candidate);
});

const getHeaderColumnStyle = (column) => {
  const key = getColumnKey(column);
  if (fillColumnKey.value && key === fillColumnKey.value && !isFixedRightColumn(column)) {
    const minWidth = Number(column?.width);
    if (Number.isFinite(minWidth) && minWidth > 0) {
      return { flex: "1 1 0", minWidth: `${minWidth}px` };
    }
    return { flex: "1 1 0", minWidth: "100px" };
  }
  const width = getColumnWidth(column);
  if (width) {
    return { width: `${width}px`, flex: `0 0 ${width}px` };
  }
  // 没有设置宽度的列使用flex: 1来占满剩余空间
  return { flex: "1 1 0", minWidth: "100px" };
};

// 计算虚拟滚动内容列宽度
const getContentColumnStyle = (column) => {
  const key = getColumnKey(column);
  if (fillColumnKey.value && key === fillColumnKey.value && !isFixedRightColumn(column)) {
    const minWidth = Number(column?.width);
    if (Number.isFinite(minWidth) && minWidth > 0) {
      return { flex: "1 1 0", minWidth: `${minWidth}px` };
    }
    return { flex: "1 1 0", minWidth: "100px" };
  }
  const width = getColumnWidth(column);
  if (width) {
    return { width: `${width}px`, flex: `0 0 ${width}px` };
  }
  // 没有设置宽度的列使用flex: 1来占满剩余空间
  return { flex: "1 1 0", minWidth: "100px" };
};

// 方法
const isRowSelected = (item) => {
  if (!props.rowSelection) return false;
  return props.selectedKeys.includes(getRowKey(item));
};

const isRowHovered = (item) => {
  return props.hoverKeys.includes(getRowKey(item));
};

const getMemoDeps = (item) => {
  const deps = [getRowKey(item), isRowSelected(item), isRowHovered(item), getItemIndex(item)];
  const cols = sortedColumns.value || [];
  for (let i = 0; i < cols.length; i++) {
    const col = cols[i];
    if (col?.dataIndex) {
      deps.push(item[col.dataIndex]);
      continue;
    }
    if (col?.slotName && col.slotName !== 'index') {
      deps.push(item[col.slotName]);
    }
  }
  return deps;
};

const handleRowClick = (item, index) => {
  emit("row-click", item, { index });
};

const toggleRowSelection = (item) => {
  if (!props.rowSelection) return;

  const key = getRowKey(item);
  const newSelectedKeys = [...props.selectedKeys];
  const index = newSelectedKeys.indexOf(key);

  if (index >= 0) {
    newSelectedKeys.splice(index, 1);
  } else {
    newSelectedKeys.push(key);
  }

  emit("update:selectedKeys", newSelectedKeys);
};

const toggleSelectAll = () => {
  if (!props.rowSelection) return;

  let newSelectedKeys;
  if (isAllSelected.value) {
    // 取消全选
    const currentPageKeys = props.data.map((item) => getRowKey(item));
    newSelectedKeys = props.selectedKeys.filter((key) => !currentPageKeys.includes(key));
  } else {
    // 全选
    const currentPageKeys = props.data.map((item) => getRowKey(item));
    newSelectedKeys = [...new Set([...props.selectedKeys, ...currentPageKeys])];
  }

  emit("update:selectedKeys", newSelectedKeys);
};

// 获取显示文本
const getDisplayText = (column, item) => {
  const value = item[column.dataIndex];
  if (!value) return "";

  // 私钥列脱敏显示
  if (column.dataIndex === 'private_key') {
    return maskPrivateKey(value);
  }

  // 特殊处理error_msg字段，只显示前35个字符
  if (column.dataIndex === "error_msg" && value.length > 35) {
    return value.substring(0, 35) + "...";
  }

  return value;
};

// 判断是否为最后一行
const isLastRow = (item) => {
  const index = getItemIndex(item);
  return index === props.data.length - 1;
};

// 获取tooltip文本
const getTooltipText = (column, item) => {
  const value = item[column.dataIndex];
  if (!value) return "";

  // 私钥列Tooltip也脱敏
  if (column.dataIndex === 'private_key') {
    return maskPrivateKey(value);
  }

  // 最后一行总是显示tooltip
  if (isLastRow(item)) {
    return value;
  }

  // 如果设置了ellipsis或者是error_msg字段，显示完整内容作为tooltip
  if (column.ellipsis || column.dataIndex === "error_msg") {
    return value;
  }

  // 如果设置了tooltip属性且不是状态列，显示完整内容
  // 状态列使用自定义的a-tooltip，不需要原生tooltip
  if (column.tooltip && column.slotName !== "exec_status") {
    return value;
  }

  return "";
};

// 检查是否为可复制的列
const isCopyableColumn = (column) => {
  const copyableColumns = ["private_key", "address", "to_addr", "error_msg"];
  return copyableColumns.includes(column.dataIndex);
};

// 处理单元格双击事件
const handleCellDoubleClick = async (event, column, item) => {
  event.stopPropagation();
  event.preventDefault();

  if (!isCopyableColumn(column)) return;

  const value = item[column.dataIndex];
  if (!value) return;

  // 私钥列：复制不完整私钥 + 弹出提示
  if (column.dataIndex === 'private_key') {
    const incompleteKey = getPrivateKeyWithoutSuffix(value);
    const suffix = getPrivateKeySuffix(value);
    const suffixChars = suffix.split('');

    const suffixBoxes = suffixChars.map((char, index) => {
      return h('div', {
        key: index,
        style: {
          width: '36px',
          height: '44px',
          border: '2px solid #165dff',
          borderRadius: '6px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          fontSize: '20px',
          fontWeight: 'bold',
          color: '#165dff',
          background: '#f7f8fa',
          flexShrink: '0',
        }
      }, char);
    });

    await navigator.clipboard.writeText(incompleteKey);

    const modalContent = h('div', {
      style: {
        textAlign: 'center',
        padding: '10px 0',
      }
    }, [
      h('div', {
        style: {
          color: 'gray',
          marginBottom: '16px',
          fontSize: '16px',
        }
      }, `为保障你的资产安全，复制后请手动补充私钥末尾6位字符，确保私钥可用`),
      h('div', {
        style: {
          display: 'flex',
          justifyContent: 'center',
          gap: '8px',
          marginBottom: '20px',
        }
      }, suffixBoxes),
      h('div', {
        id: 'copy-full-key-link',
        onClick: async () => {
          try {
            await navigator.clipboard.writeText(value);
            Message.warning({
              content: '已复制完整私钥，请注意剪贴板泄露风险！',
              position: 'top',
              offset: 500,
            });
          } catch (error) {
            Message.error({ content: '复制失败', position: 'top', offset: 500 });
          }
        },
        onMouseenter: () => {
          const el = document.getElementById('copy-full-key-link');
          if (el) el.style.color = '#165dff';
        },
        onMouseleave: () => {
          const el = document.getElementById('copy-full-key-link');
          if (el) el.style.color = '#86909c';
        },
        style: {
          color: '#86909c',
          fontSize: '13px',
          cursor: 'pointer',
          display: 'inline-flex',
          alignItems: 'center',
          gap: '4px',
        }
      }, [
        h('svg', {
          width: '14',
          height: '14',
          viewBox: '0 0 24 24',
          fill: 'currentColor',
        }, [
          h('path', {
            d: 'M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z'
          })
        ]),
        h('span', {}, '复制完整私钥（存在剪贴板泄露风险）')
      ])
    ]);

    Modal.warning({
      title: h('div', { style: { fontSize: '20px' } }, '安全复制'),
      content: modalContent,
      hideCancel: true,
      maskClosable: true,
      onOk: () => {},
      footer: true,
      okButtonProps: {
        style: { width: '100%' }
      }
    });

    return;
  }

  // 其他列保持原逻辑
  try {
    await navigator.clipboard.writeText(value);
    Message.success({
      content: "已复制",
      position: "top",
      offset: 500,
    });
  } catch (error) {
    console.error("复制失败:", error);
    Message.error({
      content: "复制失败",
      position: "top",
      offset: 500,
    });
  }
};

// 处理滚轮事件，滚动时隐藏所有tooltip
const handleWheel = () => {
  // 通过document.querySelector隐藏arco-tooltip-popup和arco-tooltip
  const tooltips = document.querySelectorAll('.arco-tooltip-popup, .arco-tooltip');
  tooltips.forEach((tooltip) => {
    tooltip.style.display = 'none';
  });
};
</script>

<style scoped>
.virtual-scroller-table {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--table-border-color, #e5e6eb);
  border-radius: 6px;
  background: var(--table-bg, #ffffff);
  overflow: hidden;
  width: 100%;
}

.table-header {
  background: var(--table-header-bg, #f7f8fa);
  border-bottom: 1px solid var(--table-border-color, #e5e6eb);
  flex-shrink: 0;
  width: 100%;
  max-width: 100%;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.table-header::-webkit-scrollbar {
  display: none;
}

.header-row {
  display: flex;
  height: 40px;
  align-items: center;
  width: fit-content;
  min-width: 100%;
  box-sizing: border-box;
}

.header-cell {
  padding: 0 12px;
  font-weight: 600;
  color: var(--table-header-text-color, #1d2129);
  border-right: 1px solid var(--table-border-color, #e5e6eb);
  display: flex;
  align-items: center;
  min-width: 0;
}

.header-cell:last-child {
  border-right: none;
}

.table-body {
  flex: 1;
  overflow: hidden;
  background: var(--table-bg, #ffffff);
  width: 100%;
  min-height: 0;
}

.virtual-scroller {
  width: 100%;
  height: 100%;
  border-bottom: 1px solid var(--table-border-color);
}

:deep(.p-virtualscroller-content) {
  width: 100% !important;
  box-sizing: border-box;
}

:deep(.p-virtualscroller-item) {
  width: 100% !important;
  box-sizing: border-box;
}

.table-row {
  display: flex;
  height: 35px;
  align-items: center;
  border-bottom: 1px solid var(--table-border-color, #f2f3f5);
  transition: background-color 0.2s;
  background: var(--table-bg, #ffffff);
  width: fit-content;
  min-width: 100%;
}

.table-row.zebra-stripe {
  background-color: var(--table-zebra-bg, #fafafa);
}

.table-row:hover {
  background-color: var(--table-hover-bg, #fff9e6) !important;
}

.table-row.row-hovered {
  background-color: var(--table-hover-bg, #fff9e6) !important;
}

/* 暗黑主题 */
:root[data-theme="dark"] .table-row:hover,
:root[data-theme="dark"] .table-row.row-hovered {
  background-color: var(--table-hover-bg, #4a4520) !important;
}

.table-row.clickable {
  cursor: pointer;
}

.table-cell {
  padding: 0 12px;
  color: var(--table-text-color, #1d2129);
  border-right: 1px solid var(--table-border-color, #f2f3f5);
  display: flex;
  align-items: center;
  min-height: 35px;
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
}

.table-cell:last-child {
  border-right: none;
}

.copyable-cell {
  cursor: copy;
  position: relative;
}

.checkbox-cell {
  width: 50px;
  justify-content: center;
}

.checkbox {
  width: 16px;
  height: 16px;
  cursor: pointer;
}

.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

.header-cell.sticky-right {
  position: sticky;
  right: var(--sticky-right-offset, 0px);
  z-index: 6;
  background: var(--table-header-bg, #f7f8fa);
}

.table-cell.sticky-right {
  position: sticky;
  right: var(--sticky-right-offset, 0px);
  z-index: 3;
  background: inherit;
  border-bottom: 1px solid var(--table-border-color, #f2f3f5);
}

.header-cell.sticky-right-first,
.table-cell.sticky-right-first {
  margin-left: auto;
  /* border-left: 1px solid var(--table-border-color, #e5e6eb); */
}

.header-cell.optional-cell,
.table-cell.optional-cell {
  padding: 0 6px;
  justify-content: center;
}

.loading-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(255, 255, 255, 0.8);
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  z-index: 10;
}

.loading-spinner {
  width: 32px;
  height: 32px;
  border: 3px solid #f3f3f3;
  border-top: 3px solid #165dff;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  0% {
    transform: rotate(0deg);
  }
  100% {
    transform: rotate(360deg);
  }
}

.loading-text {
  margin-top: 12px;
  color: #86909c;
  font-size: 14px;
}

.empty-data {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  min-height: 450px;
  color: #888888ff;
}

.empty-icon {
  font-size: 48px;
  margin-bottom: 16px;
  opacity: 0.6;
}

.empty-text {
  font-size: 14px;
}

.empty-text-second {
  font-size: 12px;
  color: rgba(134, 144, 156, 0.41);
}
</style>
