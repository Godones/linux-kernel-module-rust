import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np

sns.set_theme(style="white")

domain_crossings = [1, 2, 3, 4, 5, 6, 7, 8]
x_positions = np.arange(len(domain_crossings))

high_load_domain = [1, 0.989, 0.985, 0.989, 0.986, 0.981, 0.982, 0.985]
low_load_domain  = [0.985, 0.981, 0.976, 0.9725, 0.97, 0.957, 0.95, 0.94]

bar_width = 0.4

plt.figure(figsize=(8, 5))


colors = plt.get_cmap("tab10").colors
low_color = "#ff7f0e"   # 橙色
high_color = "#1f77b4"  # 蓝色


# ====== 柱状图（彩色 + hatch，便于黑白打印区分） ======
plt.bar(x_positions - bar_width/2, low_load_domain,
        width=bar_width, color=low_color, edgecolor="black",
        hatch="////", label="Low-load domain(512B)")

plt.bar(x_positions + bar_width/2, high_load_domain,
        width=bar_width, color=high_color, edgecolor="black",
        hatch="\\\\\\", label="High-load domain(4096B)")

# ====== 折线图（颜色 + 不同 marker） ======
plt.plot(x_positions - bar_width/2, low_load_domain,
         color=low_color, marker="o", linestyle="-",
         linewidth=1.5, markersize=6, label="Low-load trend")

plt.plot(x_positions + bar_width/2, high_load_domain,
         color=high_color, marker="^", linestyle="--",
         linewidth=1.5, markersize=6, label="High-load trend")

# ====== 动态 y 轴范围 ======
all_data = np.array(low_load_domain + high_load_domain)
y_min, y_max = all_data.min(), all_data.max()
margin = (y_max - y_min) * 0.1
plt.ylim(y_min - margin, y_max + margin)

plt.yticks(np.linspace(round(y_min - margin, 3),
                       round(y_max + margin, 3), 7))

# ====== 坐标轴样式 ======
plt.xlabel("Number of cross-domain calls or function calls", color="black")
plt.ylabel("Relative Performance with Isolation vs. No Isolation", color="black")
plt.xticks(x_positions, domain_crossings)

plt.legend(frameon=False)

ax = plt.gca()
for spine in ax.spines.values():
    spine.set_color("black")
ax.tick_params(axis="both", colors="black", direction="in")

plt.tight_layout()
plt.savefig("multi-domain.pdf", format="pdf")
plt.show()
