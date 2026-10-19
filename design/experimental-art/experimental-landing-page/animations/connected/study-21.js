export default {
 name:'Brushed reflection',family:'Surface',description:'A stretched reflection follows the pearl along the cube, like light on brushed metal.',rule:'The streak points toward the pearl and shortens as its distance decreases.',
 setup({add,faceEffects,set}){const nodes=faceEffects.map(g=>add(g,'ellipse',{fill:'#f3e6ff',ry:7}));return ({i,x,y,dx,dy,distance,angle,strength})=>set(nodes[i],{cx:x+dx*.25,cy:y+dy*.25,rx:25+distance*.18,transform:`rotate(${angle*180/Math.PI} ${x+dx*.25} ${y+dy*.25})`,opacity:strength*.3});}
};
