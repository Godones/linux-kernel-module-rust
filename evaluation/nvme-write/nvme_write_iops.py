import seaborn as sns
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

# time, iops
# 1000, 4113, 1, 0, 0
# 1999, 4321, 1, 0, 0

def get_iops_with_time(filename: str)-> list:
    with open(filename, 'r') as f:
        lines = f.readlines()
    
    iops_data = []
    for line in lines:
        time, iops = line.strip().split(',')[0:2]
        iops_data.append((int(time), int(iops)))

    return iops_data


def refactor_iops_data(multi_impl:dict)->pd.DataFrame:
    # key: impl_name
    # value: iops list
    data = {
        "Time": [],
        "IOPS": [],
        "Impl": []
    }
    for impl in multi_impl:
        for time, iops in multi_impl[impl]:
            data["Time"].append(time/1000)
            data["IOPS"].append(iops)
            data["Impl"].append(impl)
    pd_data = pd.DataFrame(data)
    return pd_data


# key: impl_name
# value: iops list
def plot_iops_with_time(multi_impl: pd.DataFrame):
    plt.figure(figsize=(10, 6))
    sns.set_theme(style="whitegrid")
    sns.lineplot(x="Time", y="IOPS", hue="Impl", data=multi_impl)
    plt.xlabel('Time(s)', fontsize=12)
    plt.ylabel('IOPS', fontsize=12)
    # plt.title('IOPS with Time', fontsize=14)

    ax = plt.gca()
    ax.grid(False)
    for ytick in ax.get_yticks():
        ax.axhline(
            y=ytick,
            color='gray',
            linestyle='--',
            linewidth=1,
            alpha=0.2,
            zorder=0.5  # 确保虚线在数据曲线后面/下面
        )

    plt.legend(fontsize=12)
    plt.tight_layout()
    plt.savefig("nvme_write_iops.svg", format="svg")
    plt.show()

    
    
if __name__ == "__main__":
    impl = ["c-lkm","rust-lkm","rust-dm"]
    map_name=  ["C", "unmodified Rust", "domain-adapted"]
    impl_data = {}
    for (idx,i) in enumerate(impl):
        impl_data[map_name[idx]] = get_iops_with_time(f"{i}-nvme-write-iops.txt")
    # print(impl_data)
    pd = refactor_iops_data(impl_data)
    print(pd)
    plot_iops_with_time(pd)