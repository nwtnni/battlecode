use fnv::{FnvHashMap, FnvHashSet};
use std::collections::BTreeSet;

use engine::controller::*;
use engine::location::*;
use engine::unit::*;
use navigate::*;

#[derive(Debug, Eq, PartialEq)]
enum Type { Star, Prime }
type Karbonite = FnvHashMap<MapLocation, u32>;

fn loc(unit: &Unit) -> MapLocation {
    unit.location().map_location().unwrap()
}

pub fn assign_rockets(nav: &mut Navigator, gc: &GameController, fin_rockets: &Vec<Unit>,
    workers: &Vec<Unit>, knights: &Vec<Unit>, rangers: &Vec<Unit>, healers: &Vec<Unit>) -> FnvHashSet<u16> {

    let mut boarding = FnvHashSet::default();
    let mut assignments = FnvHashSet::default();

    let worker_spots = fin_rockets.iter()
        .filter(|rocket| {
            let garrison = rocket.structure_garrison().unwrap();
            garrison.iter().all(|&unit| {
                gc.unit(unit).unwrap().unit_type() != UnitType::Worker
            }) && garrison.len() < rocket.structure_max_capacity().unwrap()
        });

    for spot in worker_spots {
        let closest = workers.iter()
            .filter(|worker| !boarding.contains(&worker.id()))
            .min_by_key(|worker| {
                nav.moves_between(&loc(worker), &loc(spot))
            });
        if let Some(worker) = closest {
            boarding.insert(worker.id());
            assignments.insert(spot.id());
            nav.navigate(worker, &loc(spot));
        }
    }

    let soldier_spots = fin_rockets.iter()
        .filter_map(|rocket| {
            let garrison = rocket.structure_garrison().unwrap();
            let worker = if assignments.contains(&rocket.id()) { 1 } else { 0 };
            let needed = rocket.structure_max_capacity().unwrap() - garrison.len() - worker;
            if needed > 0 { Some((needed, rocket)) } else { None }
        }).collect::<Vec<_>>();

    let soldiers = knights.iter().chain(rangers.iter()).chain(healers.iter()).collect::<Vec<_>>();
    for (needed, spot) in soldier_spots {
        let mut closest = soldiers.iter()
            .filter(|soldier| !boarding.contains(&soldier.id()))
            .collect::<Vec<_>>();
        closest.sort_by_key(|soldier| {
            nav.moves_between(&loc(soldier), &loc(&spot))
        });
        for soldier in closest.iter().take(needed) {
            boarding.insert(soldier.id());
            nav.navigate(soldier, &loc(spot));
        }
    }

    boarding
}

pub fn assign_workers(nav: &mut Navigator, workers: &Vec<Unit>, karbonite: &Karbonite,
    un_facts: &Vec<Unit>, fin_facts: &Vec<Unit>, un_rockets: &Vec<Unit>) {

    if workers.len() <= 0 { return }
    let karbonite = karbonite.keys().collect::<Vec<_>>();
    let un_facts = un_facts.iter().map(|fact| loc(fact)).collect::<Vec<_>>();
    let fin_facts = fin_facts.iter()
        .filter(|fact| fact.health() < fact.max_health())
        .map(|fact| loc(fact))
        .collect::<Vec<_>>();
    let un_rockets = un_rockets.iter().map(|rocket| loc(rocket)).collect::<Vec<_>>();

    let mut locations = Vec::new();
    let mut optimize = Vec::new();
    for worker in workers {
        let mut row = Vec::new();
        let worker_loc = loc(worker);
        for location in &karbonite {
            let priority = 5 + nav.moves_between(&worker_loc, &location) as i16;
            row.push(priority);
        }
        for location in &un_facts {
            let neighbors = nav.neighbors(&location);
            let priority = nav.moves_between(&worker_loc, &location) as i16;
            for _ in 0..neighbors { row.push(priority); locations.push(location); }
        }
        for location in &fin_facts {
            let neighbors = nav.neighbors(&location) - 1;
            let priority = 10 + nav.moves_between(&worker_loc, &location) as i16;
            for _ in 0..neighbors { row.push(priority); locations.push(location); }
        }
        for location in &un_rockets {
            let neighbors = nav.neighbors(&location);
            let priority = nav.moves_between(&worker_loc, &location) as i16;
            for _ in 0..neighbors { row.push(priority); locations.push(location); }
        }
        optimize.push(row);
    }

    let k = karbonite.len();
    if optimize.len() > 0 && optimize[0].len() > 0 {
        for (worker, location) in hungarian(optimize) {
            if location < k {
                nav.navigate(&workers[worker], &karbonite[location]);
            } else {
                nav.navigate(&workers[worker], &locations[location - k]);
            }
        }
    }
}

fn hungarian(mut matrix: Vec<Vec<i16>>) -> FnvHashMap<usize, usize> {
    let rows = matrix.len();
    let cols = matrix[0].len();
    let target = if rows < cols { rows } else { cols };

    // Reduce rows
    for mut row in &mut matrix {
        let min = row.iter().min().unwrap().clone();
        row.iter_mut().for_each(|cost| *cost -= min);
    }

    let mut mask = FnvHashMap::default();
    let mut row_cover = vec![false; rows];
    let mut col_cover = vec![false; cols];

    // Star zeros
    for row in 0..rows {
        for col in 0..cols {
            if matrix[row][col] == 0
            && !(row_cover[row] || col_cover[col]) {
                mask.insert((row, col), Type::Star);
                row_cover[row] = true;
                col_cover[col] = true;
            }
        }
    }

    // Reset cover
    row_cover.iter_mut().for_each(|cov| *cov = false);
    col_cover.iter_mut().for_each(|cov| *cov = false);
    let mut verify = true;

    loop {

        // Count cover
        if verify {
            for row in 0..rows {
                for col in 0..cols {
                    if let Some(&Type::Star) = mask.get(&(row, col)) {
                        col_cover[col] = true;
                    }
                }
            }

            if col_cover.iter().filter(|&&cov| cov).count() == target {
                return mask.into_iter()
                    .filter(|&(_, ref t)| t == &Type::Star)
                    .map(|((row, col), _)| (row, col))
                    .collect::<FnvHashMap<_, _>>()
            }
        }

        // Find uncovered zero
        let mut uncovered = None;
        for row in 0..rows {
            if uncovered != None { break }
            for col in 0..cols {
                if matrix[row][col] == 0
                && mask.get(&(row, col)).is_none()
                && !row_cover[row]
                && !col_cover[col] {
                    uncovered = Some((row, col));
                    break
                }
            }
        }

        // Add and subtract minimum uncovered value
        if let None = uncovered {
            let mut min = i16::max_value();
            for row in 0..rows {
                for col in 0..cols {
                    if row_cover[row] || col_cover[col] { continue }
                    let value = matrix[row][col];
                    min = if value < min { value } else { min };
                }
            }

            for row in 0..rows {
                for col in 0..cols {
                    if row_cover[row] { matrix[row][col] += min }
                    if !col_cover[col] { matrix[row][col] -= min }
                }
            }

            verify = false;
            continue
        }

        let (row, col) = uncovered.unwrap();
        mask.insert((row, col), Type::Prime);

        let starred = (0..cols).filter(|&col| {
            mask.get(&(row, col)) == Some(&Type::Star)
            && matrix[row][col] == 0
        }).nth(0);

        if let Some(adj) = starred {
            row_cover[row] = true;
            col_cover[adj] = false;
            verify = false;
            continue
        }

        // Alternating path of Stars and Primes
        let mut path = vec![(row, col)];
        loop {
            let (_, prev_col) = path[path.len() - 1];
            let next_star = (0..rows).filter(|&row| {
                mask.get(&(row, prev_col)) == Some(&Type::Star)
            }).nth(0);

            if let None = next_star { break }
            let star_row = next_star.unwrap();
            path.push((star_row, prev_col));

            let prime_col = (0..cols).filter(|&col| {
                mask.get(&(star_row, col)) == Some(&Type::Prime)
            }).nth(0).unwrap();
            path.push((star_row, prime_col));
        }

        // Augment path
        for (row, col) in path {
            match mask.get(&(row, col)) {
                None => continue,
                Some(&Type::Star) => mask.remove(&(row, col)),
                Some(&Type::Prime) => mask.insert((row, col), Type::Star),
            };
        }

        // Reset cover
        row_cover.iter_mut().for_each(|cov| *cov = false);
        col_cover.iter_mut().for_each(|cov| *cov = false);

        // Erase primes
        mask.retain(|_, t| t != &mut Type::Prime);
        verify = true;
    }
}

#[cfg(test)]
mod tests {
    use karbonite::*;

    #[test]
    fn test_basic() {
        let matrix = vec![
            vec![1, 1, 1],
            vec![1, 1, 1],
            vec![1, 1, 1],
        ];
        println!("{:#?}", hungarian(matrix));
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
        println!("{:?}", hungarian(matrix));
    }

    #[test]
    fn test_wikihow() {
        let matrix = vec![
            vec![10, 19, 8, 15, 19],
            vec![10, 18, 7, 17, 19],
            vec![13, 16, 9, 14, 19],
            vec![12, 19, 8, 18, 19],
            vec![14, 17, 10, 19, 19]
        ];
        hungarian(matrix);
    }
}
