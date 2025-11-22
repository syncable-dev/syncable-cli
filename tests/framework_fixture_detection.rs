use std::path::Path;

use syncable_cli::analyzer::analyze_project;

struct Case<'a> {
    name: &'a str,
    path: &'a str,
    expected_primary: &'a str,
}

#[test]
fn detects_framework_across_10_fixtures() {
    let cases = [
        Case { name: "nextjs", path: "tests/fixtures/js_frameworks/nextjs", expected_primary: "Next.js" },
        Case { name: "tanstack-start", path: "tests/fixtures/js_frameworks/tanstack-start", expected_primary: "Tanstack Start" },
        Case { name: "sveltekit", path: "tests/fixtures/js_frameworks/sveltekit", expected_primary: "SvelteKit" },
        Case { name: "nuxt", path: "tests/fixtures/js_frameworks/nuxt", expected_primary: "Nuxt.js" },
        Case { name: "astro", path: "tests/fixtures/js_frameworks/astro", expected_primary: "Astro" },
        Case { name: "solidstart", path: "tests/fixtures/js_frameworks/solidstart", expected_primary: "SolidStart" },
        Case { name: "react-router-spa", path: "tests/fixtures/js_frameworks/react-router-spa", expected_primary: "React Router v7" },
        Case { name: "angular", path: "tests/fixtures/js_frameworks/angular", expected_primary: "Angular" },
        Case { name: "expo", path: "tests/fixtures/js_frameworks/expo", expected_primary: "Expo" },
        Case { name: "express", path: "tests/fixtures/js_frameworks/express", expected_primary: "Express.js" },
    ];

    for case in cases {
        let analysis = analyze_project(Path::new(case.path))
            .unwrap_or_else(|e| panic!("{}: analysis failed: {}", case.name, e));

        let mut found = None;
        for tech in &analysis.technologies {
            if tech.name == case.expected_primary {
                found = Some(tech);
                break;
            }
        }

        if let Some(primary) = found {
            assert!(primary.is_primary, "{}: {} detected but not marked primary", case.name, case.expected_primary);
        } else {
            panic!("{}: expected to detect primary framework {} but did not. Detected: {:?}", case.name, case.expected_primary, analysis.technologies.iter().map(|t| t.name.clone()).collect::<Vec<_>>());
        }
    }
}
