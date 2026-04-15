import { ref, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Notification } from '@arco-design/web-vue';

export function useTransfer(options = {}) {
  const {
    data,
    form,
    chainValue,
    currentCoin,
    threadCount,
    enableMultiThread,
    transferConfig,
    showProgress,
    startLoading,
    stopFlag,
    stopStatus,
    transferStartTime,
    transferSessionCompleted,
    updateTransferProgress,
    checkGasPriceForTransfer,
    startGasPriceMonitoring,
    transferPaused,
    pausedTransferData,
    executeTransfer,
    windowId,
  } = options;

  const retryInProgress = ref(false);
  const retryResults = ref([]);
  const MAX_RETRY_ROUNDS = 3;
  const currentRetryRound = ref(0);
  const lastTransferStartTime = ref(0);

  async function performIntelligentRetry(failedData) {
    if (!transferStartTime.value) {
      console.warn('未找到转账开始时间，使用传统重试方式');
      executeTransfer(failedData);
      return;
    }

    // 检查是否为新一轮任务（根据开始时间变化判断）
    if (transferStartTime.value !== lastTransferStartTime.value) {
      currentRetryRound.value = 0;
      lastTransferStartTime.value = transferStartTime.value;
    }

    if (currentRetryRound.value >= MAX_RETRY_ROUNDS) {
      Notification.warning({
        content: `已达到最大重试次数 (${MAX_RETRY_ROUNDS}次)，停止重试以防止死循环`,
        position: 'topLeft',
      });
      stopStatus.value = true;
      transferSessionCompleted.value = true;
      return;
    }

    currentRetryRound.value++;
    retryInProgress.value = true;
    retryResults.value = [];

    Notification.info({
      content: `开始第 ${currentRetryRound.value}/${MAX_RETRY_ROUNDS} 轮智能重试检查，共 ${failedData.length} 笔失败交易`,
      position: 'topLeft',
    });

    try {
      const retryList = [];
      const concurrency = 5; // 并发检查数

      for (let i = 0; i < failedData.length; i += concurrency) {
        if (stopFlag.value) break;
        const chunk = failedData.slice(i, i + concurrency);
        await Promise.all(
          chunk.map(async (item) => {
            try {
              // 确定用于校验的金额
              // 只有当转账类型为"固定数量"(type='2')时，才能进行精确金额校验
              // 其他类型(全部/随机等)金额不固定，只校验地址
              let amountToCheck = null;
              if (transferConfig.value && transferConfig.value.transfer_type === '2') {
                if (form.amount_from === '1') {
                  amountToCheck = item.amount ? String(item.amount) : null;
                } else {
                  amountToCheck = form.send_count ? String(form.send_count) : null;
                }
              }

              const hasRecentTransfer = await checkRecentTransfer(
                item.private_key,
                item.to_addr,
                transferStartTime.value,
                amountToCheck
              );

              if (hasRecentTransfer) {
                const realIndex = data.value.findIndex(
                  (dataItem) => dataItem.key === item.key
                );
                if (realIndex !== -1) {
                  data.value[realIndex].error_msg =
                    '检测到链上已有相关交易，跳过重试';
                  data.value[realIndex].exec_status = '2';
                  data.value[realIndex].retry_flag = false;
                }
                retryResults.value.push({
                  key: item.key,
                  address: item.to_addr,
                  action: '跳过重试',
                  reason: '检测到链上已有相关交易',
                });
              } else {
                retryList.push(item);
                retryResults.value.push({
                  key: item.key,
                  address: item.to_addr,
                  action: '加入重试',
                  reason: '未检测到相关链上交易',
                });
              }
            } catch (error) {
              console.error(`检查交易失败 ${item.to_addr}:`, error);
              retryList.push(item);
              retryResults.value.push({
                key: item.key,
                address: item.to_addr,
                action: '加入重试',
                reason: '检查失败，保守重试',
              });
            }
          })
        );
      }

      retryInProgress.value = false;

      if (stopFlag.value) {
        Notification.warning({ content: '重试检查已停止', position: 'topLeft' });
        stopStatus.value = true;
        return;
      }

      if (retryList.length > 0) {
        Notification.info({
          content: `智能检查完成，将重试 ${retryList.length} 笔交易`,
          position: 'topLeft',
        });
        
        // 重置待重试数据的状态
        retryList.forEach((item) => {
          const realIndex = data.value.findIndex((d) => d.key === item.key);
          if (realIndex !== -1) {
            data.value[realIndex].exec_status = '0';
            data.value[realIndex].error_msg = `等待重试 (${currentRetryRound.value}/${MAX_RETRY_ROUNDS})...`;
          }
        });

        // 传入 false 以避免重置 transferStartTime
        executeTransfer(retryList, false);
      } else {
        Notification.success({
          content:
            '智能重试检查完成，所有失败交易均已确认成功或跳过',
          position: 'topLeft',
        });
        stopStatus.value = true;
        transferSessionCompleted.value = true;
      }
    } catch (error) {
      console.error('智能重试检查失败:', error);
      retryInProgress.value = false;
      Notification.error({
        content: '智能重试检查异常，停止重试',
        position: 'topLeft',
      });
      stopStatus.value = true;
      transferSessionCompleted.value = true;
    }
  }

  async function ensureBalance(item) {
    if (form.send_type !== '1' && form.send_type !== '4') return;

    const isBase = currentCoin.value.coin_type === 'base';
    const balanceKey = isBase ? 'plat_balance' : 'coin_balance';
    
    // 检查是否有有效余额
    if (item[balanceKey] !== '' && item[balanceKey] !== null && item[balanceKey] !== undefined) {
        return;
    }

    const address = item.address;
    if (!address) return;

    try {
        if (isBase) {
             const bal = await invoke('query_balance', { 
                 chain: chainValue.value, 
                 address: address 
             });
             item.plat_balance = bal;
             const realIndex = data.value.findIndex(d => d.key === item.key);
             if (realIndex !== -1) {
                 data.value[realIndex].plat_balance = bal;
             }
        } else {
             const params = {
                 chain: chainValue.value,
                 coin_config: {
                     coin_type: currentCoin.value.coin_type,
                     contract_address: currentCoin.value.contract_address || null,
                     abi: currentCoin.value.abi || null
                 },
                 items: [{
                     key: item.key,
                     address: address,
                     private_key: null,
                     plat_balance: null,
                     coin_balance: null,
                     nonce: null,
                     exec_status: '0',
                     error_msg: null,
                     retry_flag: false
                 }],
                 only_coin_config: true,
                 thread_count: 1,
                 window_id: windowId?.value || null,
             };
             
             const result = await invoke('query_balances_simple', { params });
             if (result && result.success && result.items && result.items.length > 0) {
                 const resItem = result.items[0];
                 item.coin_balance = resItem.coin_balance;
                 const realIndex = data.value.findIndex(d => d.key === item.key);
                 if (realIndex !== -1) {
                     data.value[realIndex].coin_balance = resItem.coin_balance;
                 }
             }
        }
    } catch (e) {
        console.error("自动查询余额失败", e);
    }
  }

  async function checkRecentTransfer(privateKey, targetAddress, startTime, amount = null) {
    try {
      const result = await invoke('sol_check_recent_transfers', {
        chain: chainValue.value,
        private_key: privateKey,
        target_address: targetAddress, // Solana addresses are case-sensitive, no toLowerCase()
        start_timestamp: startTime,
        coin_type: currentCoin.value.coin_type,
        contract_address:
            currentCoin.value.coin_type === 'token'
                ? currentCoin.value.contract_address
                : null,
        amount: amount,
      });

      return result.has_recent_transfer || false;
    } catch (error) {
      console.error('查询链上交易失败:', error);
      throw error;
    }
  }

  function getTransferAmount(item) {
    if (form.send_type === '2') {
      // 指定数量
      if (form.amount_from === '1') {
        return item.amount && item.amount.trim() !== ''
          ? Number(item.amount)
          : 0;
      } else {
        return form.send_count && form.send_count.trim() !== ''
          ? Number(form.send_count)
          : 0;
      }
    } else if (form.send_type === '3') {
      // 范围随机
      const min = Number(form.send_min_count) || 0;
      const max = Number(form.send_max_count) || 0;
      const precision = !isNaN(parseFloat(form.amount_precision))
        ? Number(form.amount_precision)
        : 6;

      if (min === 0 && max === 0) return 0;

      // 生成随机数
      const random = Math.random() * (max - min) + min;
      // 处理精度
      return Number(random.toFixed(precision));
    } else if (form.send_type === '4') {
      // 剩余随机
      const min = Number(form.send_min_count) || 0;
      const max = Number(form.send_max_count) || 0;
      const precision = !isNaN(parseFloat(form.amount_precision))
        ? Number(form.amount_precision)
        : 6;
      
      let balance = 0;
      // 根据币种类型获取余额
      if (currentCoin.value && currentCoin.value.coin_type === 'base') {
        balance = (item.plat_balance !== '' && item.plat_balance !== undefined && item.plat_balance !== null) ? Number(item.plat_balance) : 0;
      } else {
        balance = (item.coin_balance !== '' && item.coin_balance !== undefined && item.coin_balance !== null) ? Number(item.coin_balance) : 0;
      }
      
      if (balance <= 0) return 0;
      
      // 生成随机保留数
      const retain = Math.random() * (max - min) + min;
      
      // 计算转账金额 = 余额 - 保留数
      // 注意：如果是SOL转账，这里未精确扣除Gas，实际剩余会比retain略少(少一个Gas费)
      // 但由于保留范围通常较大，这通常是可以接受的
      let amount = balance - retain;
      if (amount < 0) amount = 0;
      
      return Number(amount.toFixed(precision));
    } else if (form.send_type === '1') {
      // 全部发送
      if (currentCoin.value && currentCoin.value.coin_type === 'base') {
        // Native SOL: Send -1 to trigger backend "Send All" logic (Precise fee calculation)
        return -1;
      }
      
      let balance = (item.coin_balance !== '' && item.coin_balance !== undefined && item.coin_balance !== null) ? Number(item.coin_balance) : 0;
      
      if (balance <= 0) return 0;
      
      // Token Transfer: Send All means sending the entire balance (no gas deduction from token amount)
      // Pass -1 to backend to handle "Send All" for tokens as well, ensuring precision
      return -1;
    }
    return 0;
  }

  async function transferFnc(inputData) {
    await iterTransfer(inputData)
        .then(async () => {
          if (stopFlag.value) {
            Notification.warning({
              content: '已停止执行！',
              position: 'topLeft',
            });
          } else {
            const retryData = inputData.filter(
                (item) => item.retry_flag === true,
            );
            if (form.error_retry === '1' && retryData.length > 0) {
              await performIntelligentRetry(retryData);
            } else {
              const successCount = inputData.filter(
                  (item) => item.exec_status === '2',
              ).length;
              const totalCount = inputData.length;

              if (successCount > 0) {
                Notification.success({
                  content: `执行完成！成功转账 ${successCount}/${totalCount} 笔`,
                  position: 'topLeft',
                });
              } else {
                Notification.warning({
                  content: '执行完成，但没有成功的转账',
                  position: 'topLeft',
                });
              }

              stopStatus.value = true;
              transferSessionCompleted.value = true;
            }
          }
          startLoading.value = false;
          stopFlag.value = false;
          showProgress.value = false;
        })
        .catch(() => {
          Notification.error({ content: '执行失败！', position: 'topLeft' });
          startLoading.value = false;
          stopStatus.value = true;
          showProgress.value = false;
        });
  }

  async function iterTransfer(accountData) {
    const isFuryMode = threadCount.value > 90;

    if (isFuryMode) {
      console.log('[狂暴模式] 已激活，线程数:', threadCount.value);
      Notification.info({
        content: '狂暴模式已激活：交易将快速批量提交，然后统一确认结果',
        position: 'topLeft',
      });

      await iterTransferFuryMode(accountData);
      return;
    }

    const isSingleThread = enableMultiThread === '0' || enableMultiThread === false;

    if (isSingleThread) {
      for (let index = 0; index < accountData.length; index++) {
        if (stopFlag.value) {
          stopStatus.value = true;
          return;
        }

        const item = accountData[index];

        if (item.exec_status !== '0') {
          continue;
        }

        // 自动查询余额逻辑
        await ensureBalance(item);

        if (form.max_gas_price && form.max_gas_price.trim()) {
          const gasPriceOk = await checkGasPriceForTransfer();
          if (!gasPriceOk) {
            pausedTransferData.value = { accountData, index };
            await startGasPriceMonitoring();

            while (transferPaused.value && !stopFlag.value) {
              await new Promise((resolve) =>
                  setTimeout(resolve, 1000),
              );
            }

            if (stopFlag.value) {
              stopStatus.value = true;
              return;
            }
          }
        }

        const realIndex = data.value.findIndex(
            (dataItem) => dataItem.key === item.key,
        );
        if (realIndex === -1) {
          console.error('无法找到对应的数据项');
          continue;
        }

        const config = {
          ...transferConfig.value,
          transfer_amount: getTransferAmount(item),
        };

        try {
          if (currentCoin.value.coin_type === 'base') {
            data.value[realIndex].exec_status = '1';
            try {
              const res = await invoke('sol_transfer', {
                index: realIndex + 1,
                item: item,
                config: config,
              });

              if (typeof res === 'object' && res !== null) {
                if (res.success && res.tx_hash) {
                  data.value[realIndex].exec_status = '2';
                  data.value[realIndex].error_msg = res.tx_hash;
                } else {
                  data.value[realIndex].exec_status = '3';
                  data.value[realIndex].error_msg =
                      res.error || '转账失败';
                }
              } else {
                data.value[realIndex].exec_status = '2';
                data.value[realIndex].error_msg = String(
                    res || '转账成功',
                );
              }
              updateTransferProgress();
            } catch (err) {
              if (err === 'base gas price 超出最大值限制') {
                Notification.error({
                  content: 'base gas price 超出最大值限制',
                  position: 'topLeft',
                });
                stopTransfer();
                data.value[realIndex].exec_status = '0';
                data.value[realIndex].error_msg = '';
                return;
              } else {
                data.value[realIndex].exec_status = '3';
                data.value[realIndex].error_msg = err;
                data.value[realIndex].retry_flag = true;
                updateTransferProgress();
              }
            }
          } else if (currentCoin.value.coin_type === 'token') {
            data.value[realIndex].exec_status = '1';
            try {
              const res = await invoke('sol_token_transfer', {
                index: realIndex + 1,
                item: item,
                config: {
                  ...config,
                  contract_address: currentCoin.value.contract_address,
                  abi: null, // Solana doesn't use ABI for standard SPL tokens
                },
              });

              if (typeof res === 'object' && res !== null) {
                if (res.success && res.tx_hash) {
                  data.value[realIndex].exec_status = '2';
                  data.value[realIndex].error_msg = res.tx_hash;
                } else {
                  data.value[realIndex].exec_status = '3';
                  data.value[realIndex].error_msg =
                      res.error || '转账失败';
                }
              } else {
                data.value[realIndex].exec_status = '2';
                data.value[realIndex].error_msg = String(
                    res || '转账成功',
                );
              }
              updateTransferProgress();
            } catch (err) {
              if (err === 'base gas price 超出最大值限制') {
                Notification.error({
                  content: 'base gas price 超出最大值限制',
                  position: 'topLeft',
                });
                stopTransfer();
                data.value[realIndex].exec_status = '0';
                data.value[realIndex].error_msg = '';
                return;
              } else {
                data.value[realIndex].exec_status = '3';
                data.value[realIndex].error_msg = err;
                data.value[realIndex].retry_flag = true;
                updateTransferProgress();
              }
            }
          } else {
            Notification.error({
              content: '未知币种类型',
              position: 'topLeft',
            });
            return;
          }
        } catch (e) {
          data.value[realIndex].exec_status = '3';
          data.value[realIndex].error_msg = e.message || '转账异常';
          data.value[realIndex].retry_flag = true;
          updateTransferProgress();
        }

        if (index < accountData.length - 1 && !stopFlag.value) {
          const minDelay =
              form.min_interval && form.min_interval.trim() !== ''
                  ? Number(form.min_interval) * 1000
                  : 1000;
          const maxDelay =
              form.max_interval && form.max_interval.trim() !== ''
                  ? Number(form.max_interval) * 1000
                  : 3000;
          const randomDelay =
              Math.floor(Math.random() * (maxDelay - minDelay + 1)) +
              minDelay;

          let nextPendingIndex = -1;
          for (let i = index + 1; i < accountData.length; i++) {
            if (accountData[i].exec_status === '0') {
              nextPendingIndex = data.value.findIndex(
                  (dataItem) => dataItem.key === accountData[i].key,
              );
              break;
            }
          }

          if (nextPendingIndex !== -1) {
            const originalErrorMsg = data.value[nextPendingIndex].error_msg;
            let remainingTime = Math.ceil(randomDelay / 1000);

            const countdownInterval = setInterval(() => {
              if (stopFlag.value) {
                clearInterval(countdownInterval);
                data.value[nextPendingIndex].error_msg =
                    originalErrorMsg;
                return;
              }

              data.value[nextPendingIndex].error_msg =
                  `等待中...${remainingTime}秒`;
              remainingTime--;

              if (remainingTime < 0) {
                clearInterval(countdownInterval);
                data.value[nextPendingIndex].error_msg =
                    originalErrorMsg;
              }
            }, 1000);

            await new Promise((resolve) => {
              setTimeout(() => {
                clearInterval(countdownInterval);
                resolve();
              }, randomDelay);
            });
          } else {
            await new Promise((resolve) =>
                setTimeout(resolve, randomDelay),
            );
          }
        }
      }
      return;
    }

    const keyToIndexMap = new Map();
    data.value.forEach((dataItem, index) => {
      keyToIndexMap.set(dataItem.key, index);
    });

    const walletGroups = new Map();
    accountData.forEach((item) => {
      const walletAddress = item.address || item.private_key;
      if (!walletGroups.has(walletAddress)) {
        walletGroups.set(walletAddress, []);
      }
      const realIndex = keyToIndexMap.get(item.key) ?? -1;
      walletGroups
          .get(walletAddress)
          .push({ ...item, originalIndex: 0, realIndex });
    });

    const walletGroupsArray = Array.from(walletGroups.values());

    const processWalletGroup = async (walletTransactions) => {
      for (const item of walletTransactions) {
        if (stopFlag.value) {
          stopStatus.value = true;
          return;
        }

        if (item.exec_status !== '0') {
          continue;
        }

        const realIndex = item.realIndex;
        if (realIndex === -1) {
          console.error('无法找到对应的数据项');
          continue;
        }
        
        // 自动查询余额逻辑
        await ensureBalance(item);

        const config = {
          ...transferConfig.value,
          transfer_amount: getTransferAmount(item),
        };

        try {
          if (currentCoin.value.coin_type === 'base') {
            data.value[realIndex].exec_status = '1';
            try {
              const res = await invoke('sol_transfer', {
                index: realIndex + 1,
                item: item,
                config: config,
              });

              if (typeof res === 'object' && res !== null) {
                if (res.success && res.tx_hash) {
                  data.value[realIndex].exec_status = '2';
                  data.value[realIndex].error_msg = res.tx_hash;
                } else {
                  data.value[realIndex].exec_status = '3';
                  data.value[realIndex].error_msg =
                      res.error || '转账失败';
                }
              } else {
                data.value[realIndex].exec_status = '2';
                data.value[realIndex].error_msg = String(
                    res || '转账成功',
                );
              }
              updateTransferProgress();
            } catch (err) {
              if (err === 'base gas price 超出最大值限制') {
                Notification.error({
                  content: 'base gas price 超出最大值限制',
                  position: 'topLeft',
                });
                stopTransfer();
                data.value[realIndex].exec_status = '0';
                data.value[realIndex].error_msg = '';
                return;
              } else {
                data.value[realIndex].exec_status = '3';
                data.value[realIndex].error_msg = err;
                data.value[realIndex].retry_flag = true;
                updateTransferProgress();
              }
            }
          } else if (currentCoin.value.coin_type === 'token') {
            data.value[realIndex].exec_status = '1';
            try {
              const res = await invoke('sol_token_transfer', {
                index: realIndex + 1,
                item: item,
                config: {
                  ...config,
                  contract_address: currentCoin.value.contract_address,
                  abi: null,
                },
              });

              if (typeof res === 'object' && res !== null) {
                if (res.success && res.tx_hash) {
                  data.value[realIndex].exec_status = '2';
                  data.value[realIndex].error_msg = res.tx_hash;
                } else {
                  data.value[realIndex].exec_status = '3';
                  data.value[realIndex].error_msg =
                      res.error || '转账失败';
                }
              } else {
                data.value[realIndex].exec_status = '2';
                data.value[realIndex].error_msg = String(
                    res || '转账成功',
                );
              }
              updateTransferProgress();
            } catch (err) {
              if (err === 'base gas price 超出最大值限制') {
                Notification.error({
                  content: 'base gas price 超出最大值限制',
                  position: 'topLeft',
                });
                stopTransfer();
                data.value[realIndex].exec_status = '0';
                data.value[realIndex].error_msg = '';
                return;
              } else {
                data.value[realIndex].exec_status = '3';
                data.value[realIndex].error_msg = err;
                data.value[realIndex].retry_flag = true;
                updateTransferProgress();
              }
            }
          } else {
            Notification.error({
              content: '未知币种类型',
              position: 'topLeft',
            });
            return;
          }
        } catch (e) {
          data.value[realIndex].exec_status = '3';
          data.value[realIndex].error_msg = e.message || '转账异常';
          data.value[realIndex].retry_flag = true;
          updateTransferProgress();
        }
      }
    };

    const workQueue = [...walletGroupsArray];
    const runningTasks = new Set();
    const maxConcurrency = Math.min(
        threadCount.value,
        walletGroupsArray.length,
    );

    const startWorkerTask = async () => {
      while (workQueue.length > 0 && !stopFlag.value) {
        const walletGroup = workQueue.shift();
        if (!walletGroup) break;

        const taskPromise = processWalletGroup(walletGroup);
        runningTasks.add(taskPromise);

        taskPromise.finally(() => {
          runningTasks.delete(taskPromise);
        });

        await taskPromise;
      }
    };

    const workers = [];
    for (let i = 0; i < maxConcurrency; i++) {
      workers.push(startWorkerTask());
    }

    await Promise.all(workers);
  }

  async function iterTransferFuryMode(accountData) {
    console.log('[狂暴模式] 开始执行，待处理数据数量:', accountData.length);

    const keyToIndexMap = new Map();
    data.value.forEach((dataItem, index) => {
      keyToIndexMap.set(dataItem.key, index);
    });

    const pendingTransactions = [];
    const submissionFinished = { value: false }; // 使用对象引用传递状态

    const walletGroups = new Map();
    accountData.forEach((item) => {
      const walletAddress = item.address || item.private_key;
      if (!walletGroups.has(walletAddress)) {
        walletGroups.set(walletAddress, []);
      }
      const realIndex = keyToIndexMap.get(item.key) ?? -1;
      walletGroups.get(walletAddress).push({ ...item, realIndex });
    });

    const walletGroupsArray = Array.from(walletGroups.values());
    const maxConcurrency = Math.min(
        threadCount.value,
        walletGroupsArray.length,
    );

    console.log('[狂暴模式] 钱包分组数:', walletGroupsArray.length, '并发数:', maxConcurrency);

    const submitWalletGroupTransactions = async (walletTransactions) => {
      for (const item of walletTransactions) {
        if (stopFlag.value) return;

        if (item.exec_status !== '0') continue;

        const realIndex = item.realIndex;
        if (realIndex === -1) {
          console.error('[狂暴模式] 无法找到对应的数据项');
          continue;
        }

        const config = {
          ...transferConfig.value,
          transfer_amount: getTransferAmount(item),
        };

        try {
          data.value[realIndex].exec_status = '1';
          data.value[realIndex].error_msg = '正在提交交易...';

          let res;
          if (currentCoin.value.coin_type === 'base') {
            res = await invoke('sol_transfer_fast', {
              index: realIndex + 1,
              item: item,
              config: config,
            });
          } else if (currentCoin.value.coin_type === 'token') {
            res = await invoke('sol_token_transfer_fast', {
              index: realIndex + 1,
              item: item,
              config: {
                ...config,
                contract_address: currentCoin.value.contract_address,
                abi: null,
              },
            });
          } else {
            throw new Error('未知币种类型');
          }

          if (res && res.success && res.tx_hash) {
            data.value[realIndex].error_msg =
                `已提交，等待确认: ${res.tx_hash.substring(0, 15)}...`;
            
            // 将交易加入待确认列表
            pendingTransactions.push({
              key: item.key,
              realIndex,
              txHash: res.tx_hash,
              item,
              config,
              retryCount: 0,
              startTime: Date.now(),
            });
          } else {
            data.value[realIndex].exec_status = '3';
            data.value[realIndex].error_msg = res?.error || '提交失败';
            data.value[realIndex].retry_flag = true;
            updateTransferProgress();
          }
        } catch (err) {
          console.error(`[狂暴模式] 提交失败: ${realIndex + 1}`, err);
          data.value[realIndex].exec_status = '3';
          data.value[realIndex].error_msg = String(err);
          data.value[realIndex].retry_flag = true;
          updateTransferProgress();
        }
      }
    };

    const workQueue = [...walletGroupsArray];
    const workers = [];

    const startSubmitWorker = async () => {
      while (workQueue.length > 0 && !stopFlag.value) {
        const walletGroup = workQueue.shift();
        if (!walletGroup) break;
        await submitWalletGroupTransactions(walletGroup);
      }
    };

    // 启动确认线程
    const confirmationWorker = async () => {
      const BATCH_SIZE = 50; // 批量查询大小
      const CHECK_INTERVAL = 1000; // 检查间隔 (ms)
      const MAX_CONFIRM_TIME = 60000; // 最大确认时间 60s (超时则标记失败)

      while (!stopFlag.value) {
        if (pendingTransactions.length === 0) {
          if (submissionFinished.value) {
            break; // 提交完成且无待确认交易，退出
          }
          await new Promise((r) => setTimeout(r, 500));
          continue;
        }

        // 处理超时交易
        const now = Date.now();
        for (let i = pendingTransactions.length - 1; i >= 0; i--) {
            if (now - pendingTransactions[i].startTime > MAX_CONFIRM_TIME) {
                const tx = pendingTransactions[i];
                data.value[tx.realIndex].exec_status = '3';
                data.value[tx.realIndex].error_msg = '确认超时';
                data.value[tx.realIndex].retry_flag = true;
                updateTransferProgress();
                pendingTransactions.splice(i, 1);
            }
        }
        
        if (pendingTransactions.length === 0) continue;

        // 批量查询
        // 我们只查询前 BATCH_SIZE 个，或者所有 pending 的 (如果支持大批量)
        // 这里为了避免并发冲突，我们取出一批进行查询，查询期间它们仍在数组中，但我们只处理这批
        // 为了防止重复频繁查询同一个，我们可以简单地轮询
        
        const batch = pendingTransactions.slice(0, BATCH_SIZE);
        if (batch.length === 0) {
             await new Promise((r) => setTimeout(r, 500));
             continue;
        }

        const txHashes = batch.map(t => t.txHash);
        
        try {
            const results = await invoke('sol_check_transactions_status_batch', {
                chain: chainValue.value,
                tx_hashes: txHashes
            });
            
            // 处理结果
            const confirmedIndices = []; // 在 batch 中的索引
            
            results.forEach((res) => {
                const txInfo = batch.find(t => t.txHash === res.hash);
                if (!txInfo) return;
                
                if (res.status.confirmed) {
                    if (res.status.success === true) {
                        data.value[txInfo.realIndex].exec_status = '2';
                        data.value[txInfo.realIndex].error_msg = txInfo.txHash;
                        data.value[txInfo.realIndex].retry_flag = false;
                    } else {
                         data.value[txInfo.realIndex].exec_status = '3';
                         data.value[txInfo.realIndex].error_msg = res.status.error || '交易执行失败';
                         data.value[txInfo.realIndex].retry_flag = true;
                    }
                    updateTransferProgress();
                    
                    // 标记为已完成，需要从 pendingTransactions 中移除
                    // 我们记录 txHash 来移除
                    const idx = pendingTransactions.findIndex(t => t.txHash === res.hash);
                    if (idx !== -1) {
                        pendingTransactions.splice(idx, 1);
                    }
                } else {
                     // 仍在确认中
                     // 可以更新 UI 显示确认中...
                     data.value[txInfo.realIndex].error_msg = `确认中... ${txInfo.txHash.substring(0, 10)}`;
                }
            });
            
        } catch (e) {
            console.error('[狂暴模式] 批量查询失败:', e);
        }

        await new Promise((r) => setTimeout(r, CHECK_INTERVAL));
      }
    };

    const confirmWorkerPromise = confirmationWorker();

    for (let i = 0; i < maxConcurrency; i++) {
      workers.push(startSubmitWorker());
    }

    await Promise.all(workers);
    submissionFinished.value = true;
    
    // 等待确认线程完成
    await confirmWorkerPromise;

    if (stopFlag.value) {
      console.log('[狂暴模式] 用户停止，中断执行');
    }
  }

  function stopTransfer() {
    startLoading.value = false;
    stopFlag.value = true;
    stopStatus.value = true;
    showProgress.value = false;
  }

  return {
    transferFnc,
    stopTransfer,
    performIntelligentRetry,
    iterTransfer,
    iterTransferFuryMode,
    retryInProgress,
    retryResults,
  };
}
