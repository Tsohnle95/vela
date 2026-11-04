export default {
 name:'Facet lift',family:'Geometry',description:'A top facet lifts by a hair as the pearl draws close.',rule:'The lifted facet moves upward by at most six icon units; a soft seam follows proximity.',
 setup({topGroups,edge,set}){return ({i,strength})=>{set(topGroups[i],{opacity:strength,transform:`translate(0 ${-strength*6})`});set(edge[i],{opacity:strength*.12});};}
};
