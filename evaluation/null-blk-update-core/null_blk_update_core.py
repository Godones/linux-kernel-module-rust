import matplotlib.pyplot as plt
import matplot2tikz

# sns.set_theme(style="white")


core_nums = [1, 2, 3, 4, 5, 6, 12, 24, 32]
x_positions = range(len(core_nums))

task_sync_time   =    [1,4,6,8,11,21,76,111,143]
stop_machine_time    = [10,13,17,19,22,34,90,122,156]

plt.figure(figsize=(8, 5))

plt.plot(x_positions, stop_machine_time,
         label='Service Downtime',
         color='black',
         marker='o',         # 圆形标记
         fillstyle='full',   # 实心标记
         linewidth=1,
         markersize=8)


# 1) Task Sync (Install) - 实心方块
plt.plot(x_positions, task_sync_time,
         label='Task Sync Time',
         color='black',
         marker='s',         # 方形标记
         fillstyle='full',   # 实心标记
         linewidth=1,
         markersize=8)


plt.xlabel("Core Number", color='black')
plt.ylabel("Time (us)", color='black')
# plt.title("The Scalability of PlugSched", color='black')
plt.xticks(x_positions, core_nums)
# plt.yticks([50,100,150,200])


plt.legend(frameon=False)
plt.grid(False)


ax = plt.gca()
for spine in ax.spines.values():
    spine.set_color('black')

ax.tick_params(axis='both', colors='black', direction='in')

plt.tight_layout()
plt.savefig("null_blk_update_core.svg", format="svg")

plt.show()
# matplot2tikz.save("null_blk_update_core.tex")



