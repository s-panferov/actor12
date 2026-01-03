use std::any::type_name;
use std::any::TypeId;
use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use dashmap::DashMap;

/// A struct to hold statistics for a given type.
#[derive(Debug)]
struct Stats {
	#[allow(dead_code)]
	type_name: &'static str,
	total: AtomicUsize,
	max_live: AtomicUsize,
	live: AtomicUsize,
}

impl Stats {
	fn new(type_name: &'static str) -> Self {
		Self {
			type_name,
			total: AtomicUsize::new(0),
			max_live: AtomicUsize::new(0),
			live: AtomicUsize::new(0),
		}
	}
}

// Global, thread-safe map to store counts for each type.
// The key is the TypeId, and the value is the statistics for that type.
static COUNTS: std::sync::OnceLock<DashMap<TypeId, Stats>> = std::sync::OnceLock::new();

// Helper function to get the global counts map
fn counts() -> &'static DashMap<TypeId, Stats> {
	COUNTS.get_or_init(DashMap::new)
}

/// A guard that increments a counter when created and decrements it when dropped.
///
/// This helps track the number of live instances of a specific type `T`.
#[derive(Debug)]
pub struct Count<T: 'static> {
	_phantom: PhantomData<T>,
}

impl<T: 'static> Count<T> {
	/// Creates a new `Count` instance, incrementing the counters for type `T`.
	pub fn new() -> Self {
		let type_id = TypeId::of::<T>();
		let type_name = type_name::<T>();

		// Get or insert the stats for this type.
		let entry = counts()
			.entry(type_id)
			.or_insert_with(|| Stats::new(type_name));

		// Increment total instances created.
		entry.total.fetch_add(1, Ordering::Relaxed);

		// Increment live instances and update max_live if necessary.
		let live = entry.live.fetch_add(1, Ordering::Relaxed) + 1;
		entry.max_live.fetch_max(live, Ordering::Relaxed);

		Self {
			_phantom: PhantomData,
		}
	}
}

impl<T: 'static> Drop for Count<T> {
	fn drop(&mut self) {
		let type_id = TypeId::of::<T>();
		if let Some(entry) = counts().get(&type_id) {
			// Decrement the live count.
			entry.live.fetch_sub(1, Ordering::Relaxed);
		}
	}
}

impl<T: 'static> Default for Count<T> {
	fn default() -> Self {
		Self::new()
	}
}

/// A snapshot of the statistics for printing.
#[allow(dead_code)]
struct Report {
	// Use BTreeMap to keep the report sorted by type name.
	by_type: BTreeMap<&'static str, (usize, usize, usize)>,
}

impl Report {
	#[allow(dead_code)]
	fn new() -> Self {
		let mut by_type = BTreeMap::new();
		for entry in counts().iter() {
			let stats = entry.value();
			by_type.insert(
				stats.type_name,
				(
					stats.total.load(Ordering::Relaxed),
					stats.max_live.load(Ordering::Relaxed),
					stats.live.load(Ordering::Relaxed),
				),
			);
		}
		Self { by_type }
	}
}

impl fmt::Display for Report {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// Determine column widths
		let name_width = self
			.by_type
			.keys()
			.map(|k| k.len())
			.max()
			.unwrap_or(0)
			.max(5); // "total" is 5 chars

		let total_width = 12;
		let max_live_width = 12;
		let live_width = 12;

		// Header
		writeln!(
			f,
			"{:<name_width$} {:>total_width$} {:>max_live_width$} {:>live_width$}",
			"", "total", "max_live", "live"
		)?;

		// Data rows
		for (name, (total, max_live, live)) in &self.by_type {
			writeln!(
				f,
				"{name:<name_width$} {total:>total_width$} {max_live:>max_live_width$} {live:>live_width$}"
			)?;
		}
		Ok(())
	}
}

/// Prints a formatted report of the current instance counts to stdout.
#[allow(dead_code)]
pub fn report() {
	println!("{}", Report::new());
}

/// Returns a string containing the formatted report of the current instance counts.
#[allow(dead_code)]
pub fn report_string() -> String {
	Report::new().to_string()
}