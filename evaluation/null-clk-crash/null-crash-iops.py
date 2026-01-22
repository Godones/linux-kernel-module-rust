
import seaborn as sns
import matplotlib.pyplot as plt
import pandas as pd

data = {
    "Test Scenario": ["No crash","5ms-crash","1ms-crash", "500us-crash", "200us-crash", "100us-crash"],
    "Throughput (IOPS/s)": [480754, 474555, 466268, 463496, 441027,417952],
}

# Convert to DataFrame
df = pd.DataFrame(data)

# Set the plot style
sns.set(style="whitegrid")

# Create a horizontal bar plot
plt.figure(figsize=(8, 4))
sns.barplot(data=df, x="Throughput (IOPS/s)", y="Test Scenario", orient="h", palette="Blues_d",)

# Add title and labels
# plt.title("FIO Throughput Across Different Scenarios", fontsize=14)
plt.xlabel("Throughput (IOPS/s)", fontsize=12)
plt.ylabel(None)
# Remove extra lines on the x-axis
# plt.gca().spines['top'].set_visible(False)
# plt.gca().spines['right'].set_visible(False)

# Adjust grid to avoid overlapping
plt.grid(visible=False)
# Display the plot
plt.tight_layout()
plt.savefig("crash-test-throughput.svg", format="svg")
plt.show()
