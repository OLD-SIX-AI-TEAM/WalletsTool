import {createApp} from "vue";
import App from "./App.vue";
import { router } from "./router";
// PrimeVue imports
import PrimeVue from 'primevue/config';
import VirtualScroller from 'primevue/virtualscroller';
import 'primeicons/primeicons.css';
// 按需导入Arco Design组件
import {
  Button,
  Table,
  TableColumn,
  Modal,
  Form,
  FormItem,
  Input,
  InputNumber,
  Select,
  Cascader,
  Option,
  Switch,
  Tabs,
  Tag,
  Progress,
  Spin,
  Alert,
  Tooltip,
  Popconfirm,
  Divider,
  Row,
  Col,
  Radio,
  RadioGroup,
  Upload,
  Textarea,
  Dropdown,
  Doption,
  InputGroup,
  Notification,
  Message,
  Popover,
  Tree,
  List,
  ListItem,
  Icon
} from '@arco-design/web-vue';
import '@arco-design/web-vue/dist/arco.css';
import "./style.css";
import {createPinia} from 'pinia'
import { useThemeStore } from './stores'

// 预加载关键资源
const preloadResources = () => {
  // 预加载字体
  const fontLink = document.createElement('link');
  fontLink.rel = 'preload';
  fontLink.as = 'font';
  fontLink.type = 'font/woff2';
  fontLink.crossOrigin = 'anonymous';
  document.head.appendChild(fontLink);

  // CSS已通过import语句正确导入，无需预加载
};

// 确保路由准备就绪
const setupRoutePreloading = () => {
  // 等待路由准备就绪
  router.isReady().then(() => {
    // 路由已准备就绪，可以执行后续操作
  });
};

// 优化字体加载
const optimizeFonts = () => {
  document.fonts.ready.then(() => {
    document.body.classList.add('fonts-loaded');
  });
};

// 添加全局错误处理
window.addEventListener('error', (event) => {
  console.error('全局JavaScript错误:', {
    message: event.message,
    filename: event.filename,
    lineno: event.lineno,
    colno: event.colno,
    error: event.error
  })
})

window.addEventListener('unhandledrejection', (event) => {
  console.error('未处理的Promise拒绝:', event.reason)
})

const pinia = createPinia()
const app = createApp(App)

app.use(pinia)
app.use(router)

const themeStore = useThemeStore()
themeStore.initTheme()

// 添加Vue错误处理
app.config.errorHandler = (err, vm, info) => {
  console.error('Vue错误处理器:', {
    error: err,
    component: vm,
    info: info
  })
}

// 注册Arco Design组件
app.use(Button)
app.use(Table)
app.use(Tabs)
app.use(TableColumn)
app.use(Modal)
app.use(Form)
app.use(FormItem)
app.use(Input)
app.use(InputNumber)
app.use(Select)
app.use(Cascader)
app.use(Option)
app.use(Switch)
app.use(Tag)
app.use(Progress)
app.use(Spin)
app.use(Alert)
app.use(Tooltip)
app.use(Popconfirm)
app.use(Divider)
app.use(Row)
app.use(Col)
app.use(Radio)
app.use(RadioGroup)
app.use(Upload)
app.use(Textarea)
app.use(Dropdown)
app.use(Doption)
app.use(InputGroup)
app.use(Popover)
app.use(Tree)
app.use(List)
app.use(ListItem)
app.use(Icon)

app.use(PrimeVue);
app.component('VirtualScroller', VirtualScroller)

// 执行预加载和优化
preloadResources();
optimizeFonts();
setupRoutePreloading();

// 禁用右键菜单
document.addEventListener('contextmenu', function(e) {
  e.preventDefault();
  return false;
});

// 禁用F12开发者工具（可选）
document.addEventListener('keydown', function(e) {
  if (e.key === 'F12' || (e.ctrlKey && e.shiftKey && e.key === 'I')) {
    e.preventDefault();
    return false;
  }
});

app.mount("#app");
