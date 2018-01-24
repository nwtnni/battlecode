use fnv::{FnvHashMap, FnvHashSet};

use engine::controller::*;

fn hungarian(mut matrix: Vec<Vec<u32>>) {
    // Reduce rows
    for mut row in &mut matrix {
        let min = row.iter().min().unwrap().clone();
        row.iter_mut().for_each(|cost| *cost -= min);
    }

    // Reduce columns
    for j in 0..matrix.len() {
        let min = matrix.iter().map(|row| row[j]).min().unwrap().clone();
        matrix.iter_mut().for_each(|row| row[j] -= min);
    }

    loop {
        // Cover greedily
        let mut cross_row = FnvHashSet::default();
        let mut cross_col = FnvHashSet::default();
        let mut assign_row = FnvHashSet::default();
        let mut assign_col = FnvHashSet::default();

        for (i, row) in matrix.iter().enumerate() {
            let j = row.iter().enumerate()
                .filter_map(|(j, value)| if *value == 0 { Some(j)} else { None })
                .filter(|_| !cross_row.contains(&i))
                .filter(|j| !cross_col.contains(j))
                .nth(0);
            if let Some(j) = j {
                cross_row.insert(i);
                cross_col.insert(j);
                assign_row.insert(i);
                assign_col.insert(j);
            }
        }

        // Draw
        let mut mark_col: FnvHashSet<usize> = FnvHashSet::default();
        let mut mark_row = (0..matrix.len())
            .filter(|row| !assign_row.contains(row))
            .collect::<FnvHashSet<_>>();
        let mut new_row = mark_row.iter().cloned().collect::<Vec<_>>();
        let mut new_col = Vec::new();

        loop {
            for &row in &new_row {
                new_col.extend((0..matrix.len())
                    .filter(|col| !mark_col.contains(col))
                    .filter(|&col| matrix[row][col] == 0));
            }

            if new_col.len() == 0 { break }
            new_row = Vec::new();
            mark_row.extend(new_row.iter());

            for &col in &new_col {
                new_row.extend((0..matrix.len())
                    .filter(|row| !mark_row.contains(row))
                    .filter(|&row| matrix[row][col] == 0));
            }

            mark_col.extend(new_col.iter());
            new_col = Vec::new();
        }

        // Check number of lines drawn
        if mark_col.len() - mark_row.len() == 0 {
            let mut assignment = Vec::new();
            for row in 0..matrix.len() {
                matrix[row].iter().filter(|col| )
            }
        }

        // Find remaining elements
        let mut unmarked = Vec::new();
        for row in 0..matrix.len() {
            for col in 0..matrix.len()  {
                if mark_row.contains(&row) && !mark_col.contains(&col) {
                    unmarked.push((row, col));
                }
            }
        }

        let min = unmarked.iter().map(|&(row, col)| matrix[row][col]).min().unwrap();
        unmarked.iter().for_each(|&(row, col)| matrix[row][col] -= min);
    }
}
