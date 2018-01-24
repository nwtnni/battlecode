use fnv::{FnvHashMap, FnvHashSet};

use engine::controller::*;

fn hungarian(mut matrix: Vec<Vec<i16>>) {
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

    println!("After reduction: ");
    for row in 0..matrix.len() {
        for col in 0..matrix.len() {
            print!("{} ", matrix[row][col]);
        }
        println!("");
    }

    loop {
        // Cover greedily
        let mut cross_row = FnvHashSet::default();
        let mut cross_col = FnvHashSet::default();
        let mut assign_row = FnvHashSet::default();
        let mut assign = FnvHashSet::default();

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
                assign.insert((i, j));
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

            mark_row.extend(new_row.iter());
            new_row = Vec::new();
            if new_col.len() == 0 { break }

            for &col in &new_col {
                new_row.extend((0..matrix.len())
                    .filter(|row| assign.contains(&(*row, col)))
                    .filter(|row| !mark_row.contains(row))
                    .filter(|&row| matrix[row][col] == 0));
            }

            mark_col.extend(new_col.iter());
            new_col = Vec::new();
        }

        println!("Values");
        for row in 0..matrix.len() {
            for col in 0..matrix.len() {
                print!("{} ", matrix[row][col]);
            }
            println!("");
        }

        println!("Lines");
        for row in 0..matrix.len() {
            for col in 0..matrix.len() {
                let v = mark_col.contains(&col);
                let h = !mark_row.contains(&row);
                if v && h {
                    print!("+ ");
                } else if v {
                    print!("| ");
                } else if h {
                    print!("- ");
                } else {
                    print!("{} ", matrix[row][col]);
                }
            }
            println!("");
        }

        // Check number of lines drawn
        if mark_col.len() + matrix.len() - mark_row.len() == matrix.len() { return }

        // Find min of remaining elements
        let mut min = i16::max_value();
        for row in 0..matrix.len() {
            for col in 0..matrix.len()  {
                if mark_row.contains(&row) && !mark_col.contains(&col) {
                    if matrix[row][col] < min { min = matrix[row][col] }
                }
            }
        }

        for row in 0..matrix.len() {
            for col in 0..matrix.len()  {
                let v = mark_col.contains(&col);
                let h = !mark_row.contains(&row);
                if !v && !h { matrix[row][col] -= min }
                else if v && h { matrix[row][col] += min }
            }
        }
    }
}

mod tests {

    use karbonite::*;

    #[test]
    fn test_basic() {
        let matrix = vec![
            vec![1, 1, 1],
            vec![1, 1, 1],
            vec![1, 1, 1],
        ];
        hungarian(matrix);
    }

    #[test]
    fn test_sales() {
        let matrix = vec![
            vec![250, 400, 350],
            vec![400, 600, 350],
            vec![200, 400, 250],
        ];
        hungarian(matrix);
    }

    #[test]
    fn test_wiki() {
        let matrix = vec![
            vec![0, 1, 2, 3],
            vec![4, 5, 6, 0],
            vec![0, 2, 4, 5],
            vec![3, 0, 0, 9],
        ];
        hungarian(matrix);
    }

    #[test]
    fn test_bulldozer() {
        let matrix = vec![
            vec![90, 75, 75, 80],
            vec![35, 85, 55, 65],
            vec![125, 95, 90, 105],
            vec![45, 110, 95, 115],
        ];
        hungarian(matrix);
    }

    #[test]
    fn test_stack() {
        let matrix = vec![
            vec![2, 9, 2, 7, 1],
            vec![6, 8, 7, 6, 1],
            vec![4, 6, 5, 3, 1],
            vec![4, 2, 7, 3, 1],
            vec![5, 3, 9, 5, 1],
        ];
        hungarian(matrix);
    }

    #[test]
    fn test_large() {
        let matrix = vec![
            vec![1,5,5,2,3,1,2,3,2,4,5,2,3,1,5,5,2,3,1,5,1,4,3,2,5],
            vec![5,5,3,2,3,2,5,1,4,3,2,5,3,2,4,5,2,5,2,1,1,4,1,2,5],
            vec![5,1,4,3,2,5,1,1,4,1,2,5,2,2,3,4,1,4,5,3,2,4,5,2,5],
            vec![1,1,4,1,2,5,3,2,4,5,2,5,5,5,1,5,1,5,5,2,2,3,4,1,4],
            vec![3,2,4,5,2,5,2,2,3,4,1,4,5,4,2,1,3,2,5,5,5,1,5,1,5],
            vec![2,2,3,4,1,4,5,5,1,5,1,5,5,5,2,5,5,1,4,5,4,2,1,3,2],
            vec![5,5,1,5,1,5,5,5,3,2,3,2,1,5,5,1,5,1,5,5,5,2,5,5,1],
            vec![5,4,2,1,3,2,5,1,4,3,2,5,5,5,4,2,1,3,2,5,1,4,3,2,5],
            vec![5,5,2,5,5,1,1,1,4,1,2,5,1,5,5,2,5,5,1,1,1,4,1,2,5],
            vec![2,4,5,3,4,2,3,2,4,5,2,5,2,2,4,5,3,4,2,3,2,4,5,2,5],
            vec![2,2,5,5,1,3,2,2,3,4,1,4,2,2,2,5,5,1,3,2,2,3,4,1,4],
            vec![4,1,5,4,5,3,5,5,1,5,1,5,5,4,1,5,4,5,3,5,5,1,5,1,5],
            vec![5,1,4,3,2,5,3,2,4,5,2,5,5,5,1,4,3,2,5,3,2,4,5,2,5],
            vec![1,1,4,1,2,5,2,2,3,4,1,4,1,1,1,4,1,2,5,2,2,3,4,1,4],
            vec![3,2,4,5,2,5,5,5,1,5,1,5,4,3,2,4,5,2,5,5,5,1,5,1,5],
            vec![2,2,3,4,1,4,5,4,2,1,3,2,1,2,2,3,4,1,4,5,4,2,1,3,2],
            vec![5,5,1,5,1,5,5,5,2,5,5,1,2,5,5,1,5,1,5,5,5,2,5,5,1],
            vec![5,1,4,3,2,5,3,5,1,4,3,2,5,3,5,2,2,3,5,2,2,3,2,5,3],
            vec![3,4,1,4,1,1,1,1,1,4,1,2,5,5,1,4,3,2,5,1,4,1,2,5,2],
            vec![1,5,5,2,3,1,5,3,2,4,5,2,5,1,1,4,1,2,5,2,4,5,2,5,5],
            vec![5,5,3,2,3,2,2,2,2,3,4,1,4,3,2,4,5,2,5,2,3,4,1,4,3],
            vec![5,1,4,3,2,5,2,5,5,1,5,1,5,2,2,3,4,1,4,5,1,5,1,5,5],
            vec![1,1,4,1,2,5,2,5,4,2,1,3,2,5,5,1,5,1,5,4,2,1,3,2,1],
            vec![3,2,4,5,2,5,1,5,5,2,5,5,1,5,4,2,1,3,2,5,2,5,5,1,3],
            vec![2,2,3,4,1,4,1,2,4,5,3,4,2,5,5,2,5,5,1,4,5,3,4,2,2],
        ];
        hungarian(matrix);
    }

    #[test]
    fn test_c() {
        let matrix = vec![
            vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0],
            vec![53,207,256,207,231,348,348,348,231,244,244,0,0,0],
            vec![240,33,67,33,56,133,133,133,56,33,33,0,0,0],
            vec![460,107,200,107,122,324,324,324,122,33,33,0,0,0],
            vec![167,340,396,340,422,567,567,567,422,442,442,0,0,0],
            vec![167,367,307,367,433,336,336,336,433,158,158,0,0,0],
            vec![160,20,37,20,31,70,70,70,31,22,22,0,0,0],
            vec![200,307,393,307,222,364,364,364,222,286,286,0,0,0],
            vec![33,153,152,153,228,252,252,252,228,78,78,0,0,0],
            vec![93,140,185,140,58,118,118,118,58,44,44,0,0,0],
            vec![0,7,22,7,19,58,58,58,19,0,0,0,0,0],
            vec![67,153,241,153,128,297,297,297,128,39,39,0,0,0],
            vec![73,253,389,253,253,539,539,539,253,36,36,0,0,0],
            vec![73,267,270,267,322,352,352,352,322,231,231,0,0,0],
        ];
        hungarian(matrix);
    }
}
