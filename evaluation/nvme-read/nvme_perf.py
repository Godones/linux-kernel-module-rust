import pickle
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np

final_data = {
        "Block Size": [],
        "QD": [],
        "Type": [],  # randread or randwrite
        "rel_iops": [],
        "Sub-Type": [],  # R-LKM-1T/ R-LKM-6T / DM-LKM-1T / DM-LKM-6T
        "iops_std": []
    }

# rel-DM-c
# rel-LKM-c
# bs512B-jb1-qd1
# bs{bs}-jb{jb}-qd{qd}

def get_config_args(config: str) -> tuple:
    """
    从配置字符串中提取线程数、块大小和队列深度
    """
    parts = config.split("-")
    qd = parts[-1][2:]
    bs = parts[0][2:]
    jb = parts[1][2:]
    return qd, bs, jb


def get_nvme_test_data()->pd.DataFrame:
    # read data from pickle
    with open("nvmerandread_iops.pkl", "rb") as f:
        ty_with_read_iops: dict = pickle.load(f)
    with open("nvmeread_iops.pkl", "rb") as f:
        ty_with_write_iops: dict = pickle.load(f)

    iops = {
        "rel-DM-c": {},
        "rel-LKM-c": {}
    }

    # join data
    for type_key in ty_with_read_iops.keys():
        for config_key in ty_with_read_iops[type_key].keys():
            if "rel-DM-c" in type_key or "rel-LKM-c" in  type_key:
                iops[type_key][config_key] = (ty_with_read_iops[type_key][config_key][0], ty_with_write_iops[type_key][config_key][0], ty_with_read_iops[type_key][config_key][2], ty_with_write_iops[type_key][config_key][2])

    df = pd.DataFrame(iops)
    # print(df)

    for key in iops.keys():
        value_dict: dict = iops[key]
        for config in value_dict.keys():
            qd, bs, jb = get_config_args(config)
            final_data["Block Size"].extend([bs, bs])
            final_data["QD"].extend([qd,qd])
            final_data["Type"].extend(["randread", "read"])
            final_data["rel_iops"].extend([value_dict[config][0], value_dict[config][1]])
            final_data["iops_std"].extend([value_dict[config][2], value_dict[config][3]])

            if key == "rel-DM-c":
                final_data["Sub-Type"].extend([format("domain-adapted-" + jb + "-thread")]*2)
            else:
                final_data["Sub-Type"].extend([format("unmodified Rust-" + jb + "-thread")]*2)


    df = pd.DataFrame(final_data)
    print(df)
    return df


def draw(data:pd.DataFrame):
    # Apply the default theme
    # sns.set_style("whitegrid")
    custom_colors = ["#DEAA79", "#FFE6A9", "#B1C29E", "#659287"]
    color_mapping = {
        "unmodified Rust-1-thread": custom_colors[0],
        "domain-adapted-1-thread": custom_colors[1],
    }


    print(color_mapping)

    grid  = sns.FacetGrid(
        data,
        col="QD",
        row="Type",
        margin_titles=False,
        sharex=True,
        sharey=True,
        despine=False,
        col_order=["1", "8", "32", "128"],
        row_order=["randread", "read"],
    )

    grid.map(
        sns.barplot, 
        "Block Size", 
        "rel_iops", 
        "Sub-Type", 
        dodge=True, 
        edgecolor="black",
        hue_order=["unmodified Rust-1-thread", "domain-adapted-1-thread"],
        palette=color_mapping,
        width=0.6,
        linewidth=0.75, 
    )
    
    ax:list[plt.Axes] = grid.axes
 
    for (i,ty)in enumerate(["randread", "read"]):
        for (j,qd) in enumerate(["1", "8", "32", "128"]):
            ax[i][j].grid(True, which="both", linestyle="-", linewidth=1, alpha=1) # 添加网格
            
            ax[i][j].tick_params(axis="x", labelrotation=45) # 设置横坐标标签旋转

             # 调整横坐标间距
            ax[i][j].set_xticks(range(len(data["Block Size"].unique())))
            ax[i][j].set_xticklabels(data["Block Size"].unique(), rotation=45)

            ax[i][j].grid(True, zorder=0)  # 网格线绘制在底层
            ax[i][j].set_axisbelow(True)  # 确保网格线在柱状图后

            # 添加误差线
            subset = data[(data["QD"] == qd) & (data["Type"] == ty)]
            # print(subset)

            # 误差数据需要交叉遍历，绘图时是按照子类型绘制的，pd数据是按照类型排序的

            # err_values = subset["iops_std"].values
            new_err_values = []
            # get unmodified Rust-1-thread, domain-adapted-1-thread, R-6T, DM-6T
            for blk_size in ["512B","4k","1m","4m","16m"]:
                for hue in ["unmodified Rust-1-thread", "domain-adapted-1-thread"]:
                    new_err_values.append(subset[(subset["Sub-Type"] == hue) & (subset["Block Size"] == blk_size)]["iops_std"].values[0])

            # print("New Error Values: ", new_err_values)

            for patch, err in zip(ax[i][j].patches, new_err_values):
                x = patch.get_x() + patch.get_width() / 2  # 柱体的中心
                y = patch.get_height()
                ax[i][j].errorbar(
                    x,
                    y,
                    yerr=err/2,
                    fmt="none",
                    ecolor="black",
                    elinewidth=0.5, #误差条宽度
                    capsize=2, # #误差条帽子大小
                )
                # print("QD: ", qd, "Type: ", ty, "Error: ", err)
            # 调整边框线宽
            for patch in ax[i][j].patches:
                plt.setp(patch, linewidth=0.5)

            # 增大组间距
            for text in ax[i][j].get_xticklabels():
                text.set_horizontalalignment("center")

            ax[i][j].set_xlabel("") 
            ax[i][j].set_ylabel("")
            if i == 0:
                ax[i][j].set_title(f"qd {qd}")
            else:
                ax[i][j].set_title("")
            
            if j == 0 and i == 0:
                ax[i][j].set_ylabel("randread")
                ax[i][j].set_ylim(-0.4, 0.2)
            if j == 0 and i == 1:
                ax[i][j].set_ylabel("read")
                # ax[i][j].set_ylim(-0.15, 0.2)
    
    
    # set title
    # grid.fig.suptitle("Nvme Blk Throughput, $(R - C) / C$ (Bare Metal)")
    
    # 添加全局 y 轴标签
    grid.fig.text(0.02, 0.5, "IO/s Relative Difference", va="center", ha="center",
            rotation=90, )

    # 添加全局横坐标
    grid.fig.text(0.5, 0.02, "Block Size (KiB)", ha="center", fontsize=12)


    grid.set_xlabels("")
    grid.set_xticklabels(labels=["512B", "4KiB", "1MiB", "4MiB", "16MiB"], rotation=45)
    # show legend
    # two clumn and tow row
    # unmodified Rust-1-thread, domain-adapted-1-thread
    # R-6T, DM-6T
    grid.add_legend(ncol=2, frameon=True, bbox_to_anchor=(0.02, 0.98), loc="outside upper left")


    # 调整布局
    plt.tight_layout(rect=[0.03, 0.04, 1, 0.9])  # 为图例和标题留出空间

    plt.savefig("performance_comparison_nvme.svg", format="svg")
    plt.show()


if __name__ == "__main__":
    nvme_df = get_nvme_test_data()
    draw(nvme_df)