const hasAp=!(new URLSearchParams(document.location.search).has("no-ap"));
if(!hasAp){
    document.querySelector(".ap_nt").innerText=" you";
    document.querySelector("#ap_switch").href="?";
}

// Modified from https://codepen.io/brinkbot/pen/oNZJXqK
(async () => {
    // fetch data and render

    const resp = await fetch(
        "courses.json"
    );
    let [data,dept] = await resp.json();

    const {disp,url,grad}=dept;

    // Test out of courses
    if(hasAp){
        data=data.map(({prereqs,...rest})=>({
            ...rest,
            prereqs: prereqs.includes(1301)?[]:prereqs
        }));

    }

    const deps=new Set(data.map(({prereqs})=>prereqs).flat());
    const filteredData=data.filter(({id,prereqs})=>deps.has(id)||prereqs.length);
    const ids=data.map(({id})=>id+"");
    const stratify=d3.dagStratify()
        .parentIds(({prereqs})=>prereqs.map(pre=>pre+"").filter(pre=>ids.includes(pre)))
        .id(({id})=>id+"");
    const dag = stratify(filteredData);
    const nodeRadius = 20;
    const layout = d3
        /*
        .grid()
        .nodeSize([3*nodeRadius,nodeRadius])
        */
        .sugiyama() // base layout
        .layering(d3.layeringLongestPath())
        .decross(d3.decrossTwoLayer())
        //.decross(d3.decrossOpt().large("large"))
        .nodeSize((node) => [(node ? 3.6 : 0.25) * nodeRadius, 3 * nodeRadius]); // set node size instead of constraining to fit
    const { width, height } = layout(dag);

    // --------------------------------
    // This code only handles rendering
    // --------------------------------
    const svgSelection = d3.select("svg");
    svgSelection.attr("viewBox", [0, 0, width, height].join(" "));
    const defs = svgSelection.append("defs"); // For gradients

    const steps = dag.size();
    const interp = d3.interpolateRainbow;
    const colorMap = new Map();
    for (const [i, node] of dag.descendants().entries()) {
        colorMap.set(node.data.id, interp(i / steps));
    }

    // How to draw edges
    const line = d3
        .line()
        .curve(d3.curveCatmullRom)
        .x((d) => d.x)
        .y((d) => d.y);

    // Plot edges
    svgSelection
        .append("g")
        .selectAll("path")
        .data(dag.links())
        .enter()
        .append("path")
        .attr("d", ({ points }) => line(points))
        .attr("fill", "none")
        .attr("stroke-width", 3)
        .attr("stroke", ({ source, target }) => {
            // encodeURIComponents for spaces, hope id doesn't have a `--` in it
            const gradId = encodeURIComponent(`${source.data.id}--${target.data.id}`);
            const grad = defs
                .append("linearGradient")
                .attr("id", gradId)
                .attr("gradientUnits", "userSpaceOnUse")
                .attr("x1", source.x)
                .attr("x2", target.x)
                .attr("y1", source.y)
                .attr("y2", target.y);
            grad
                .append("stop")
                .attr("offset", "0%")
                .attr("stop-color", colorMap.get(source.data.id));
            grad
                .append("stop")
                .attr("offset", "100%")
                .attr("stop-color", colorMap.get(target.data.id));
            return `url(#${gradId})`;
        });

    // Select nodes
    const nodes = svgSelection
        .append("g")
        .selectAll("g")
        .data(dag.descendants())
        .enter()
        .append("g")
        .attr("transform", ({ x, y }) => `translate(${x}, ${y})`);

    // Plot node circles
    nodes
        .append("circle")
        .attr("r", nodeRadius)
        .attr("fill", (n) => colorMap.get(n.data.id));

    // Add text to nodes
    nodes
        .append("a")
        .attr("href",(d)=>{
            const search=encodeURIComponent(`${dept} ${d.data.id}. ${d.data.name}.`.toLowerCase());
            return `https://catalog.gatech.edu/courses-${grad?"grad":"undergrad"}/${url}/#:~:text=${search}`
        })
        .append("text")
        .text((d) => d.data.id)
        .attr("font-weight", "bold")
        .attr("font-family", "sans-serif")
        .attr("text-anchor", "middle")
        .attr("alignment-baseline", "middle")
        .attr("fill", "white")
        .append("svg:title")
        .text(d=>d.data.name);

    save_as_svg();
})();

// Courtesy of https://stackoverflow.com/questions/23218174/how-do-i-save-export-an-svg-file-after-creating-an-svg-with-d3-js-ie-safari-an
function save_as_svg(){


    var svg_data = document.querySelector("svg").innerHTML //put id of your svg element here

    var head = '<svg title="graph" version="1.1" xmlns="http://www.w3.org/2000/svg">'

    //if you have some additional styling like graph edges put them inside <style> tag

    var style = '<style>circle {cursor: pointer;stroke-width: 1.5px;}text {font: 10px arial;}path {stroke: DimGrey;stroke-width: 1.5px;}</style>'

    var full_svg = head +  style + svg_data + "</svg>"
    var blob = new Blob([full_svg], {type: "image/svg+xml"});
    const url=URL.createObjectURL(blob);
    if(false){
        const a=document.querySelector("#viz");
        a.href=url;
        a.download="graph.svg";
    }

    //saveAs(blob, "graph.svg");


};

