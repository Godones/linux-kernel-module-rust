import seaborn as sns
import matplotlib.pyplot as plt
import numpy as np
import json
import pandas as pd


def get_bins_and_ccdf(filename: str) -> tuple:
    # Read data from JSON
    with open(filename, "r") as f:
        fio_out = json.load(f)
        bins = fio_out["jobs"][0]["write"]["lat_ns"]["bins"]

        # Extract latency values and their frequencies
        latencies = np.array(list(map(int, bins.keys()))) / 1000  # Convert to microseconds
        frequencies = np.array(list(bins.values()))
        total = frequencies.sum()

        # Calculate CCDF from frequencies
        ccdf = np.cumsum(frequencies[::-1])[::-1] / total  # Reverse cumulative sum
        return latencies, ccdf


def refactor_bins_data(multi_impl: dict) -> pd.DataFrame:
    data = {
        "Latency": [],
        "CCDF": [],
        "Impl": []
    }
    for impl in multi_impl:
        for tup in multi_impl[impl]:
            latencies, ccdf = tup
            for i in range(len(latencies)):
                data["Latency"].append(latencies[i])
                data["CCDF"].append(ccdf[i])
                data["Impl"].append(impl)
    return pd.DataFrame(data)


def draw_with_bins(multi_impl: pd.DataFrame, origin_data: dict):
    # set size of the plot
    plt.figure(figsize=(10, 6))
    sns.set_theme(style="whitegrid")
    sns.lineplot(x="Latency", y="CCDF", hue="Impl", data=multi_impl)

    # Mark key percentiles
    key_percentiles = [90, 99, 99.95]
    for key_p in key_percentiles:
        ccdf_value = (100 - key_p) / 100
        plt.axhline(y=ccdf_value, color='black', linestyle='--', linewidth=1)
        plt.text(8000, ccdf_value, f'{key_p}th percentile', color='black', fontsize=10)

    # Set custom ticks based on bins' latency values
    all_latencies = multi_impl["Latency"]
    custom_ticks = np.unique(
        np.concatenate([
            np.linspace(all_latencies.min(), 5000, 3),  # Dense ticks for small values
            np.linspace(5000, 7000, 6),  # Sparse ticks for large values
            np.linspace(7000, all_latencies.max(), 2)  # Dense ticks for large values
        ])
    )
    plt.xticks(custom_ticks, labels=[f"{int(tick)}" for tick in custom_ticks], fontsize=10)

    # rotate x-axis labels
    plt.xticks(rotation=45)
 
    plt.yscale('log')  # Keep Y-axis log scale
    plt.ylim(1e-4, 1)  # Set Y-axis range
    plt.xlabel('Latency (μs)', fontsize=12)
    plt.ylabel('CCDF', fontsize=12)
    plt.title('Tail Latency CCDF', fontsize=14)
    plt.legend(fontsize=12)
    plt.tight_layout()
    plt.savefig("nvme_write_lat.svg", format="svg")
    plt.show()


if __name__ == "__main__":
    impl = ["c-lkm", "rust-lkm", "rust-dm"]
    impl_data = {}
    for i in impl:
        data = get_bins_and_ccdf(f"{i}-nvme-write-lat.json")
        impl_data[i] = [data]
    pd_data = refactor_bins_data(impl_data)
    draw_with_bins(pd_data, impl_data)
