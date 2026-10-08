/// Categories in first-seen order, and one value row per series.
///
/// A repeated series and category adds to the value already stored. That is
/// how JFreeChart's category dataset increments.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CategoryDataset {
    categories: Vec<String>,
    series: Vec<Series>,
}

#[derive(Debug, Clone, PartialEq)]
struct Series {
    name: String,
    values: Vec<f64>,
}

impl CategoryDataset {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_value(
        &mut self,
        series: impl Into<String>,
        category: impl Into<String>,
        value: f64,
    ) {
        let series = series.into();
        let category = category.into();
        let index = if let Some(index) = self.categories.iter().position(|name| name == &category) {
            index
        } else {
            self.categories.push(category);
            for row in &mut self.series {
                row.values.push(0.0);
            }
            self.categories.len() - 1
        };
        if let Some(row) = self.series.iter_mut().find(|row| row.name == series) {
            row.values[index] += value;
        } else {
            let mut values = vec![0.0; self.categories.len()];
            values[index] = value;
            self.series.push(Series {
                name: series,
                values,
            });
        }
    }

    pub fn is_empty(&self) -> bool {
        self.series.is_empty() || self.categories.is_empty()
    }

    pub(crate) fn category_count(&self) -> usize {
        self.categories.len()
    }

    pub(crate) fn series_count(&self) -> usize {
        self.series.len()
    }

    pub(crate) fn category(&self, index: usize) -> &str {
        &self.categories[index]
    }

    pub(crate) fn series_name(&self, index: usize) -> &str {
        &self.series[index].name
    }

    pub(crate) fn value(&self, series: usize, category: usize) -> f64 {
        self.series[series].values[category]
    }

    /// Largest value a bar or line can reach. Stacked kinds use the column sum.
    pub(crate) fn peak(&self, stacked: bool) -> (f64, f64) {
        if self.is_empty() {
            return (0.0, 0.0);
        }
        let mut min_value = 0.0_f64;
        let mut max_value = 0.0_f64;
        for category in 0..self.categories.len() {
            if stacked {
                let total: f64 = (0..self.series.len())
                    .map(|series| self.value(series, category))
                    .sum();
                min_value = min_value.min(total.min(0.0));
                max_value = max_value.max(total.max(0.0));
            } else {
                for series in 0..self.series.len() {
                    let value = self.value(series, category);
                    min_value = min_value.min(value);
                    max_value = max_value.max(value);
                }
            }
        }
        (min_value, max_value)
    }
}
